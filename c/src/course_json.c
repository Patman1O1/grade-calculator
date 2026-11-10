#include "course_json.h"

#include <errno.h>
#include <math.h>
#include <stdarg.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

/* ---------------------------------------------------------------- decode --- */

typedef struct {
    char path[256];
    char *err;
    size_t err_size;
} ctx;

static bool error(ctx *c, const char *field, const char *fmt, ...)
{
    char msg[200];
    va_list ap;
    va_start(ap, fmt);
    vsnprintf(msg, sizeof msg, fmt, ap);
    va_end(ap);
    const char *where = c->path[0] ? c->path : (field ? "" : "course");
    const char *sep = c->path[0] && field ? "." : "";
    snprintf(c->err, c->err_size, "%s%s%s: %s", where, sep, field ? field : "", msg);
    return false;
}

/* Appends "name" or "[i]" to the path; returns the old length to restore. */
static size_t push_path(ctx *c, const char *fmt, ...)
{
    size_t old = strlen(c->path);
    va_list ap;
    va_start(ap, fmt);
    vsnprintf(c->path + old, sizeof c->path - old, fmt, ap);
    va_end(ap);
    return old;
}

static char *dup_string(const char *s)
{
    size_t n = strlen(s) + 1;
    char *d = malloc(n);
    if (d)
        memcpy(d, s, n);
    return d;
}

static bool want_object(ctx *c, const json_value *v)
{
    if (v->type != JSON_OBJECT)
        return error(c, NULL, "expected an object, got %s", json_type_name(v->type));
    return true;
}

static bool req_string(ctx *c, const json_value *obj, const char *key, char **out)
{
    const json_value *v = json_get(obj, key);
    if (!v)
        return error(c, key, "missing");
    if (v->type != JSON_STRING)
        return error(c, key, "expected a string, got %s", json_type_name(v->type));
    *out = dup_string(v->u.string);
    return *out ? true : error(c, key, "out of memory");
}

static bool opt_string(ctx *c, const json_value *obj, const char *key, char **out)
{
    *out = NULL;
    const json_value *v = json_get(obj, key);
    if (!v || v->type == JSON_NULL)
        return true;
    if (v->type != JSON_STRING)
        return error(c, key, "expected a string, got %s", json_type_name(v->type));
    *out = dup_string(v->u.string);
    return *out ? true : error(c, key, "out of memory");
}

static bool number_value(ctx *c, const char *key, const json_value *v, double *out)
{
    if (v->type != JSON_NUMBER)
        return error(c, key, "expected a number, got %s", json_type_name(v->type));
    if (!isfinite(v->u.number))
        return error(c, key, "number out of range");
    *out = v->u.number;
    return true;
}

static bool req_number(ctx *c, const json_value *obj, const char *key, double *out)
{
    const json_value *v = json_get(obj, key);
    if (!v)
        return error(c, key, "missing");
    return number_value(c, key, v, out);
}

/* Optional number; absent or null leaves *present false. */
static bool opt_number(ctx *c, const json_value *obj, const char *key, bool *present, double *out)
{
    *present = false;
    const json_value *v = json_get(obj, key);
    if (!v || v->type == JSON_NULL)
        return true;
    if (!number_value(c, key, v, out))
        return false;
    *present = true;
    return true;
}

static bool opt_bool(ctx *c, const json_value *obj, const char *key, bool *out)
{
    *out = false;
    const json_value *v = json_get(obj, key);
    if (!v || v->type == JSON_NULL)
        return true;
    if (v->type != JSON_BOOL)
        return error(c, key, "expected a boolean, got %s", json_type_name(v->type));
    *out = v->u.boolean;
    return true;
}

/* Drop counts: floored and clamped at 0, like Math.max(0, Math.floor(x ?? 0)). */
static bool opt_count(ctx *c, const json_value *obj, const char *key, size_t *out)
{
    bool present;
    double x;
    *out = 0;
    if (!opt_number(c, obj, key, &present, &x))
        return false;
    if (!present)
        return true;
    double d = floor(x);
    if (d <= 0)
        *out = 0;
    else if (d >= (double)SIZE_MAX)
        *out = SIZE_MAX;
    else
        *out = (size_t)d;
    return true;
}

static const json_value *req_array(ctx *c, const json_value *obj, const char *key)
{
    const json_value *v = json_get(obj, key);
    if (!v) {
        error(c, key, "missing");
        return NULL;
    }
    if (v->type != JSON_ARRAY) {
        error(c, key, "expected an array, got %s", json_type_name(v->type));
        return NULL;
    }
    return v;
}

static bool decode_item(ctx *c, const json_value *v, grade_item *it)
{
    if (!want_object(c, v))
        return false;
    if (!opt_string(c, v, "id", &it->id) || !req_string(c, v, "name", &it->name))
        return false;

    const json_value *score = json_get(v, "score");
    if (!score)
        return error(c, "score", "missing (use null for not graded)");
    if (score->type == JSON_NULL) {
        it->graded = false;
    } else {
        if (!number_value(c, "score", score, &it->score))
            return false;
        it->graded = true;
    }
    return req_number(c, v, "outOf", &it->out_of) &&
           opt_number(c, v, "worth", &it->has_worth, &it->worth) &&
           opt_bool(c, v, "excluded", &it->excluded) &&
           opt_bool(c, v, "neverDrop", &it->never_drop) &&
           opt_string(c, v, "note", &it->note);
}

static bool decode_category(ctx *c, const json_value *v, grade_category *cat)
{
    if (!want_object(c, v))
        return false;
    if (!opt_string(c, v, "id", &cat->id) || !req_string(c, v, "name", &cat->name) ||
        !req_number(c, v, "weight", &cat->weight) ||
        !opt_count(c, v, "dropLowest", &cat->drop_lowest) ||
        !opt_count(c, v, "dropHighest", &cat->drop_highest) ||
        !opt_bool(c, v, "extraCredit", &cat->extra_credit))
        return false;

    const json_value *items = req_array(c, v, "items");
    if (!items)
        return false;
    size_t n = items->u.array.count;
    if (n > 0) {
        cat->items = calloc(n, sizeof *cat->items);
        if (!cat->items)
            return error(c, "items", "out of memory");
    }
    for (size_t i = 0; i < n; i++) {
        cat->item_count = i + 1; /* so a partial item is freed too */
        size_t old = push_path(c, ".items[%zu]", i);
        if (!decode_item(c, &items->u.array.items[i], &cat->items[i]))
            return false;
        c->path[old] = '\0';
    }
    return true;
}

static bool decode_cutoff(ctx *c, const json_value *v, grade_cutoff *cut)
{
    return want_object(c, v) && req_string(c, v, "letter", &cut->letter) &&
           req_number(c, v, "min", &cut->min);
}

bool course_from_json(const json_value *v, grade_course *out, char *err, size_t err_size)
{
    ctx c = { "", err, err_size };
    memset(out, 0, sizeof *out);
    out->display_decimals = 2;

    bool ok = false;
    if (!want_object(&c, v) || !opt_string(&c, v, "id", &out->id) ||
        !req_string(&c, v, "name", &out->name) || !opt_string(&c, v, "notes", &out->notes))
        goto done;

    const json_value *scheme = json_get(v, "scheme");
    if (!scheme) {
        error(&c, "scheme", "missing");
        goto done;
    }
    if (scheme->type == JSON_STRING && strcmp(scheme->u.string, "points") == 0) {
        out->scheme = GRADE_SCHEME_POINTS;
    } else if (scheme->type == JSON_STRING && strcmp(scheme->u.string, "weighted") == 0) {
        out->scheme = GRADE_SCHEME_WEIGHTED;
    } else {
        error(&c, "scheme", "expected \"points\" or \"weighted\"");
        goto done;
    }

    bool has_decimals;
    double decimals;
    if (!opt_number(&c, v, "displayDecimals", &has_decimals, &decimals))
        goto done;
    if (has_decimals)
        out->display_decimals = decimals < 0 ? 0 : decimals > 10 ? 10 : (int)decimals;

    const json_value *cats = req_array(&c, v, "categories");
    if (!cats)
        goto done;
    size_t n = cats->u.array.count;
    if (n > 0) {
        out->categories = calloc(n, sizeof *out->categories);
        if (!out->categories) {
            error(&c, "categories", "out of memory");
            goto done;
        }
    }
    for (size_t i = 0; i < n; i++) {
        out->category_count = i + 1;
        size_t old = push_path(&c, "categories[%zu]", i);
        if (!decode_category(&c, &cats->u.array.items[i], &out->categories[i]))
            goto done;
        c.path[old] = '\0';
    }

    const json_value *scale = req_array(&c, v, "scale");
    if (!scale)
        goto done;
    n = scale->u.array.count;
    if (n > 0) {
        out->scale = calloc(n, sizeof *out->scale);
        if (!out->scale) {
            error(&c, "scale", "out of memory");
            goto done;
        }
    }
    for (size_t i = 0; i < n; i++) {
        out->scale_count = i + 1;
        size_t old = push_path(&c, "scale[%zu]", i);
        if (!decode_cutoff(&c, &scale->u.array.items[i], &out->scale[i]))
            goto done;
        c.path[old] = '\0';
    }
    ok = true;
done:
    if (!ok)
        grade_course_free(out);
    return ok;
}

/* ---------------------------------------------------------------- encode --- */

static void write_percent(FILE *out, bool has, double percent)
{
    if (has)
        json_write_number(out, percent);
    else
        fputs("null", out);
}

static const char *need_kind_name(grade_need_kind kind)
{
    switch (kind) {
    case GRADE_NEED_NO_REMAINING: return "no-remaining";
    case GRADE_NEED_SECURED: return "secured";
    case GRADE_NEED_IMPOSSIBLE: return "impossible";
    case GRADE_NEED_AVERAGE: return "needs";
    }
    return "?";
}

void course_result_write_json(FILE *out, const grade_course_result *r,
                              const needed_answer *answers, size_t answer_count)
{
    const grade_course *course = r->course;
    fputs("{\"name\":", out);
    json_write_string(out, course->name);
    fprintf(out, ",\"scheme\":\"%s\"", course->scheme == GRADE_SCHEME_POINTS ? "points" : "weighted");
    fputs(",\"earned\":", out);
    json_write_number(out, r->earned);
    fputs(",\"possible\":", out);
    json_write_number(out, r->possible);
    fputs(",\"percent\":", out);
    write_percent(out, r->has_percent, r->percent);
    fputs(",\"letter\":", out);
    if (r->letter)
        json_write_string(out, r->letter);
    else
        fputs("null", out);
    fprintf(out, ",\"remaining\":%zu,\"categories\":[", grade_remaining_count(course));

    for (size_t c = 0; c < r->category_count; c++) {
        const grade_category_result *cr = &r->categories[c];
        if (c)
            fputc(',', out);
        fputs("{\"name\":", out);
        json_write_string(out, cr->category->name);
        fputs(",\"weight\":", out);
        json_write_number(out, cr->category->weight);
        fputs(",\"earned\":", out);
        json_write_number(out, cr->earned);
        fputs(",\"possible\":", out);
        json_write_number(out, cr->possible);
        fputs(",\"percent\":", out);
        write_percent(out, cr->has_percent, cr->percent);
        fputs(",\"items\":[", out);
        for (size_t i = 0; i < cr->item_count; i++) {
            const grade_item_result *ir = &cr->items[i];
            if (i)
                fputc(',', out);
            fputs("{\"name\":", out);
            json_write_string(out, ir->item->name);
            fprintf(out, ",\"counted\":%s,\"dropped\":%s,\"excluded\":%s,\"earned\":",
                    ir->counted ? "true" : "false", ir->dropped ? "true" : "false",
                    ir->item->excluded ? "true" : "false");
            json_write_number(out, ir->earned);
            fputs(",\"possible\":", out);
            json_write_number(out, ir->possible);
            fputc('}', out);
        }
        fputs("]}", out);
    }
    fputc(']', out);

    if (answer_count > 0) {
        fputs(",\"needed\":[", out);
        for (size_t i = 0; i < answer_count; i++) {
            const needed_answer *a = &answers[i];
            if (i)
                fputc(',', out);
            fputs("{\"letter\":", out);
            if (a->label)
                json_write_string(out, a->label);
            else
                fputs("null", out);
            fputs(",\"target\":", out);
            json_write_number(out, a->target);
            fprintf(out, ",\"kind\":\"%s\"", need_kind_name(a->needed.kind));
            if (a->needed.kind == GRADE_NEED_IMPOSSIBLE) {
                fputs(",\"best\":", out);
                json_write_number(out, a->needed.value);
            } else if (a->needed.kind == GRADE_NEED_AVERAGE) {
                fputs(",\"fraction\":", out);
                json_write_number(out, a->needed.value);
            }
            fputc('}', out);
        }
        fputc(']', out);
    }
    fputs("}\n", out);
}

/* ------------------------------------------------------------------ files --- */

char *read_file(const char *path, size_t *len, char *err, size_t err_size)
{
    bool is_stdin = strcmp(path, "-") == 0;
    FILE *f = is_stdin ? stdin : fopen(path, "rb");
    if (!f) {
        snprintf(err, err_size, "%s", strerror(errno));
        return NULL;
    }
    size_t cap = 1 << 16, n = 0;
    char *data = malloc(cap);
    while (data) {
        n += fread(data + n, 1, cap - n - 1, f);
        if (n < cap - 1)
            break;
        char *bigger = realloc(data, cap * 2);
        if (!bigger) {
            free(data);
            data = NULL;
            break;
        }
        data = bigger;
        cap *= 2;
    }
    bool read_error = ferror(f);
    if (!is_stdin)
        fclose(f);
    if (!data) {
        snprintf(err, err_size, "out of memory");
        return NULL;
    }
    if (read_error) {
        free(data);
        snprintf(err, err_size, "read error");
        return NULL;
    }
    data[n] = '\0';
    *len = n;
    return data;
}
