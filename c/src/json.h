/*
 * json.h — minimal strict JSON (RFC 8259) reader and writer helpers.
 * Just enough for course files: a DOM parser and string/number output.
 */
#ifndef GRADE_JSON_H
#define GRADE_JSON_H

#include <stdbool.h>
#include <stddef.h>
#include <stdio.h>

typedef enum { JSON_NULL, JSON_BOOL, JSON_NUMBER, JSON_STRING, JSON_ARRAY, JSON_OBJECT } json_type;

typedef struct json_value json_value;
typedef struct json_member json_member;

struct json_value {
    json_type type;
    union {
        bool boolean;
        double number;
        char *string; /* NUL-terminated UTF-8; embedded NULs are rejected */
        struct {
            json_value *items;
            size_t count;
        } array;
        struct {
            json_member *members;
            size_t count;
        } object;
    } u;
};

struct json_member {
    char *key;
    json_value value;
};

/* Parses `len` bytes of `text`. On failure returns false and writes a message
 * with line and column to `err`. On success free `out` with json_free. */
bool json_parse(const char *text, size_t len, json_value *out, char *err, size_t err_size);
void json_free(json_value *value);

/* Member of an object by key (last one wins, as in JavaScript), or NULL. */
const json_value *json_get(const json_value *object, const char *key);

const char *json_type_name(json_type type);

/* Writes a JSON string literal with escaping. */
void json_write_string(FILE *out, const char *s);
/* Writes a number that round-trips exactly; non-finite values become null. */
void json_write_number(FILE *out, double x);

#endif /* GRADE_JSON_H */
