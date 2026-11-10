#include "json.h"

#include <math.h>
#include <stdarg.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#define MAX_DEPTH 256

typedef struct {
    const char *text;
    size_t len;
    size_t pos;
    char *err;
    size_t err_size;
    bool failed;
} parser;

static void fail(parser *p, const char *fmt, ...)
{
    if (p->failed)
        return;
    p->failed = true;
    size_t line = 1, col = 1;
    for (size_t i = 0; i < p->pos && i < p->len; i++) {
        if (p->text[i] == '\n') {
            line++;
            col = 1;
        } else {
            col++;
        }
    }
    char msg[200];
    va_list ap;
    va_start(ap, fmt);
    vsnprintf(msg, sizeof msg, fmt, ap);
    va_end(ap);
    snprintf(p->err, p->err_size, "line %zu, column %zu: %s", line, col, msg);
}

static void skip_ws(parser *p)
{
    while (p->pos < p->len) {
        char c = p->text[p->pos];
        if (c != ' ' && c != '\t' && c != '\n' && c != '\r')
            break;
        p->pos++;
    }
}

static bool literal(parser *p, const char *word)
{
    size_t n = strlen(word);
    if (p->len - p->pos >= n && memcmp(p->text + p->pos, word, n) == 0) {
        p->pos += n;
        return true;
    }
    return false;
}

/* ---------------------------------------------------------------- strings --- */

typedef struct {
    char *data;
    size_t len;
    size_t cap;
} buffer;

static bool buf_push(buffer *b, const char *s, size_t n)
{
    if (b->len + n + 1 > b->cap) {
        size_t cap = b->cap ? b->cap * 2 : 32;
        while (cap < b->len + n + 1)
            cap *= 2;
        char *d = realloc(b->data, cap);
        if (!d)
            return false;
        b->data = d;
        b->cap = cap;
    }
    memcpy(b->data + b->len, s, n);
    b->len += n;
    b->data[b->len] = '\0';
    return true;
}

static int hex4(const char *s)
{
    int v = 0;
    for (int i = 0; i < 4; i++) {
        char c = s[i];
        v <<= 4;
        if (c >= '0' && c <= '9')
            v |= c - '0';
        else if (c >= 'a' && c <= 'f')
            v |= c - 'a' + 10;
        else if (c >= 'A' && c <= 'F')
            v |= c - 'A' + 10;
        else
            return -1;
    }
    return v;
}

static size_t utf8_encode(uint32_t cp, char out[4])
{
    if (cp < 0x80) {
        out[0] = (char)cp;
        return 1;
    }
    if (cp < 0x800) {
        out[0] = (char)(0xC0 | (cp >> 6));
        out[1] = (char)(0x80 | (cp & 0x3F));
        return 2;
    }
    if (cp < 0x10000) {
        out[0] = (char)(0xE0 | (cp >> 12));
        out[1] = (char)(0x80 | ((cp >> 6) & 0x3F));
        out[2] = (char)(0x80 | (cp & 0x3F));
        return 3;
    }
    out[0] = (char)(0xF0 | (cp >> 18));
    out[1] = (char)(0x80 | ((cp >> 12) & 0x3F));
    out[2] = (char)(0x80 | ((cp >> 6) & 0x3F));
    out[3] = (char)(0x80 | (cp & 0x3F));
    return 4;
}

/* Parses a string starting at the opening quote. Returns a malloc'd copy. */
static char *parse_string(parser *p)
{
    buffer b = { 0 };
    p->pos++; /* opening quote */
    if (!buf_push(&b, "", 0))
        goto oom;
    while (p->pos < p->len) {
        unsigned char c = (unsigned char)p->text[p->pos];
        if (c == '"') {
            p->pos++;
            return b.data;
        }
        if (c < 0x20) {
            fail(p, "control character in string");
            goto error;
        }
        if (c != '\\') {
            size_t start = p->pos;
            while (p->pos < p->len && p->text[p->pos] != '"' && p->text[p->pos] != '\\' &&
                   (unsigned char)p->text[p->pos] >= 0x20)
                p->pos++;
            if (!buf_push(&b, p->text + start, p->pos - start))
                goto oom;
            continue;
        }
        if (p->pos + 1 >= p->len)
            break;
        char e = p->text[p->pos + 1];
        p->pos += 2;
        const char *simple = NULL;
        switch (e) {
        case '"': simple = "\""; break;
        case '\\': simple = "\\"; break;
        case '/': simple = "/"; break;
        case 'b': simple = "\b"; break;
        case 'f': simple = "\f"; break;
        case 'n': simple = "\n"; break;
        case 'r': simple = "\r"; break;
        case 't': simple = "\t"; break;
        case 'u': break;
        default:
            p->pos -= 2;
            fail(p, "invalid escape \\%c", e);
            goto error;
        }
        if (simple) {
            if (!buf_push(&b, simple, 1))
                goto oom;
            continue;
        }
        if (p->len - p->pos < 4 || hex4(p->text + p->pos) < 0) {
            fail(p, "invalid \\u escape");
            goto error;
        }
        uint32_t cp = (uint32_t)hex4(p->text + p->pos);
        p->pos += 4;
        if (cp >= 0xD800 && cp <= 0xDBFF) {
            int lo = -1;
            if (p->len - p->pos >= 6 && p->text[p->pos] == '\\' && p->text[p->pos + 1] == 'u')
                lo = hex4(p->text + p->pos + 2);
            if (lo < 0xDC00 || lo > 0xDFFF) {
                fail(p, "unpaired UTF-16 surrogate");
                goto error;
            }
            p->pos += 6;
            cp = 0x10000 + ((cp - 0xD800) << 10) + ((uint32_t)lo - 0xDC00);
        } else if (cp >= 0xDC00 && cp <= 0xDFFF) {
            fail(p, "unpaired UTF-16 surrogate");
            goto error;
        } else if (cp == 0) {
            fail(p, "\\u0000 is not supported");
            goto error;
        }
        char utf8[4];
        if (!buf_push(&b, utf8, utf8_encode(cp, utf8)))
            goto oom;
    }
    fail(p, "unterminated string");
    goto error;
oom:
    fail(p, "out of memory");
error:
    free(b.data);
    return NULL;
}

/* ---------------------------------------------------------------- numbers --- */

static bool is_digit(char c)
{
    return c >= '0' && c <= '9';
}

static bool parse_number(parser *p, double *out)
{
    size_t start = p->pos;
    const char *t = p->text;
    size_t i = p->pos;
    if (i < p->len && t[i] == '-')
        i++;
    if (i < p->len && t[i] == '0') {
        i++;
    } else if (i < p->len && t[i] >= '1' && t[i] <= '9') {
        while (i < p->len && is_digit(t[i]))
            i++;
    } else {
        fail(p, "invalid number");
        return false;
    }
    if (i < p->len && t[i] == '.') {
        i++;
        if (i >= p->len || !is_digit(t[i])) {
            p->pos = i;
            fail(p, "digit expected after decimal point");
            return false;
        }
        while (i < p->len && is_digit(t[i]))
            i++;
    }
    if (i < p->len && (t[i] == 'e' || t[i] == 'E')) {
        i++;
        if (i < p->len && (t[i] == '+' || t[i] == '-'))
            i++;
        if (i >= p->len || !is_digit(t[i])) {
            p->pos = i;
            fail(p, "digit expected in exponent");
            return false;
        }
        while (i < p->len && is_digit(t[i]))
            i++;
    }
    size_t n = i - start;
    char small[64];
    char *copy = n < sizeof small ? small : malloc(n + 1);
    if (!copy) {
        fail(p, "out of memory");
        return false;
    }
    memcpy(copy, t + start, n);
    copy[n] = '\0';
    /* The grammar above already guarantees a valid C number in the "C" locale. */
    *out = strtod(copy, NULL);
    if (copy != small)
        free(copy);
    p->pos = i;
    return true;
}

/* ------------------------------------------------------------------ values --- */

static bool parse_value(parser *p, json_value *out, int depth);

static bool parse_array(parser *p, json_value *out, int depth)
{
    out->type = JSON_ARRAY;
    out->u.array.items = NULL;
    out->u.array.count = 0;
    size_t cap = 0;
    p->pos++; /* [ */
    skip_ws(p);
    if (p->pos < p->len && p->text[p->pos] == ']') {
        p->pos++;
        return true;
    }
    for (;;) {
        if (out->u.array.count == cap) {
            size_t ncap = cap ? cap * 2 : 4;
            json_value *items = realloc(out->u.array.items, ncap * sizeof *items);
            if (!items) {
                fail(p, "out of memory");
                return false;
            }
            out->u.array.items = items;
            cap = ncap;
        }
        /* Count the slot first so a partially parsed value is still freed. */
        json_value *slot = &out->u.array.items[out->u.array.count++];
        slot->type = JSON_NULL;
        if (!parse_value(p, slot, depth + 1))
            return false;
        skip_ws(p);
        if (p->pos < p->len && p->text[p->pos] == ',') {
            p->pos++;
            continue;
        }
        if (p->pos < p->len && p->text[p->pos] == ']') {
            p->pos++;
            return true;
        }
        fail(p, "expected ',' or ']'");
        return false;
    }
}

static bool parse_object(parser *p, json_value *out, int depth)
{
    out->type = JSON_OBJECT;
    out->u.object.members = NULL;
    out->u.object.count = 0;
    size_t cap = 0;
    p->pos++; /* { */
    skip_ws(p);
    if (p->pos < p->len && p->text[p->pos] == '}') {
        p->pos++;
        return true;
    }
    for (;;) {
        skip_ws(p);
        if (p->pos >= p->len || p->text[p->pos] != '"') {
            fail(p, "expected string key");
            return false;
        }
        if (out->u.object.count == cap) {
            size_t ncap = cap ? cap * 2 : 4;
            json_member *m = realloc(out->u.object.members, ncap * sizeof *m);
            if (!m) {
                fail(p, "out of memory");
                return false;
            }
            out->u.object.members = m;
            cap = ncap;
        }
        json_member *m = &out->u.object.members[out->u.object.count];
        m->key = parse_string(p);
        if (!m->key)
            return false;
        m->value.type = JSON_NULL; /* so a failure below frees cleanly */
        out->u.object.count++;
        skip_ws(p);
        if (p->pos >= p->len || p->text[p->pos] != ':') {
            fail(p, "expected ':'");
            return false;
        }
        p->pos++;
        if (!parse_value(p, &m->value, depth + 1))
            return false;
        skip_ws(p);
        if (p->pos < p->len && p->text[p->pos] == ',') {
            p->pos++;
            continue;
        }
        if (p->pos < p->len && p->text[p->pos] == '}') {
            p->pos++;
            return true;
        }
        fail(p, "expected ',' or '}'");
        return false;
    }
}

static bool parse_value(parser *p, json_value *out, int depth)
{
    out->type = JSON_NULL;
    if (depth > MAX_DEPTH) {
        fail(p, "nesting deeper than %d", MAX_DEPTH);
        return false;
    }
    skip_ws(p);
    if (p->pos >= p->len) {
        fail(p, "unexpected end of input");
        return false;
    }
    char c = p->text[p->pos];
    switch (c) {
    case '{':
        return parse_object(p, out, depth);
    case '[':
        return parse_array(p, out, depth);
    case '"':
        out->u.string = parse_string(p);
        if (!out->u.string)
            return false;
        out->type = JSON_STRING;
        return true;
    case 't':
        if (literal(p, "true")) {
            out->type = JSON_BOOL;
            out->u.boolean = true;
            return true;
        }
        break;
    case 'f':
        if (literal(p, "false")) {
            out->type = JSON_BOOL;
            out->u.boolean = false;
            return true;
        }
        break;
    case 'n':
        if (literal(p, "null"))
            return true;
        break;
    default:
        if (c == '-' || is_digit(c)) {
            if (!parse_number(p, &out->u.number))
                return false;
            out->type = JSON_NUMBER;
            return true;
        }
    }
    fail(p, "unexpected character '%c'", c);
    return false;
}

bool json_parse(const char *text, size_t len, json_value *out, char *err, size_t err_size)
{
    parser p = { text, len, 0, err, err_size, false };
    if (err_size > 0)
        err[0] = '\0';
    /* Skip a UTF-8 byte order mark. */
    if (len >= 3 && memcmp(text, "\xEF\xBB\xBF", 3) == 0)
        p.pos = 3;
    if (!parse_value(&p, out, 0)) {
        json_free(out);
        return false;
    }
    skip_ws(&p);
    if (p.pos != p.len) {
        fail(&p, "unexpected data after the JSON value");
        json_free(out);
        return false;
    }
    return true;
}

void json_free(json_value *v)
{
    if (!v)
        return;
    switch (v->type) {
    case JSON_STRING:
        free(v->u.string);
        break;
    case JSON_ARRAY:
        for (size_t i = 0; i < v->u.array.count; i++)
            json_free(&v->u.array.items[i]);
        free(v->u.array.items);
        break;
    case JSON_OBJECT:
        for (size_t i = 0; i < v->u.object.count; i++) {
            free(v->u.object.members[i].key);
            json_free(&v->u.object.members[i].value);
        }
        free(v->u.object.members);
        break;
    default:
        break;
    }
    v->type = JSON_NULL;
}

const json_value *json_get(const json_value *object, const char *key)
{
    if (!object || object->type != JSON_OBJECT)
        return NULL;
    for (size_t i = object->u.object.count; i-- > 0;)
        if (strcmp(object->u.object.members[i].key, key) == 0)
            return &object->u.object.members[i].value;
    return NULL;
}

const char *json_type_name(json_type type)
{
    switch (type) {
    case JSON_NULL: return "null";
    case JSON_BOOL: return "boolean";
    case JSON_NUMBER: return "number";
    case JSON_STRING: return "string";
    case JSON_ARRAY: return "array";
    case JSON_OBJECT: return "object";
    }
    return "?";
}

/* ----------------------------------------------------------------- writing --- */

void json_write_string(FILE *out, const char *s)
{
    fputc('"', out);
    for (const unsigned char *c = (const unsigned char *)s; *c; c++) {
        switch (*c) {
        case '"': fputs("\\\"", out); break;
        case '\\': fputs("\\\\", out); break;
        case '\b': fputs("\\b", out); break;
        case '\f': fputs("\\f", out); break;
        case '\n': fputs("\\n", out); break;
        case '\r': fputs("\\r", out); break;
        case '\t': fputs("\\t", out); break;
        default:
            if (*c < 0x20)
                fprintf(out, "\\u%04x", *c);
            else
                fputc(*c, out);
        }
    }
    fputc('"', out);
}

void json_write_number(FILE *out, double x)
{
    if (!isfinite(x)) {
        fputs("null", out);
        return;
    }
    /* Shortest of %.15g..%.17g that reads back as the same double. */
    char buf[40];
    for (int prec = 15; prec <= 17; prec++) {
        snprintf(buf, sizeof buf, "%.*g", prec, x);
        if (strtod(buf, NULL) == x)
            break;
    }
    fputs(buf, out);
}
