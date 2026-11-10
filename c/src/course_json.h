/*
 * course_json.h — course JSON (the web app's export format) <-> engine structs.
 */
#ifndef GRADE_COURSE_JSON_H
#define GRADE_COURSE_JSON_H

#include <stdbool.h>
#include <stddef.h>
#include <stdio.h>

#include "grade.h"
#include "json.h"

/* Decodes one course object. Validation follows the web app's import check
 * (src/storage.ts): name, scheme, categories and scale are required; every
 * category needs name, weight and items; every item needs name, score
 * (number or null) and outOf. Unknown keys are ignored.
 * On failure returns false with a message naming the offending path, e.g.
 * "categories[1].items[3].outOf: expected a number, got string". */
bool course_from_json(const json_value *value, grade_course *out, char *err, size_t err_size);

/* A needed-score question and its answer, for output. */
typedef struct {
    const char *label;   /* letter, or NULL for a custom target */
    double target;       /* percent */
    grade_needed needed;
} needed_answer;

/* Writes one result as a single-line JSON object (see c/README.md). */
void course_result_write_json(FILE *out, const grade_course_result *result,
                              const needed_answer *answers, size_t answer_count);

/* Reads a whole file ("-" for stdin) into a NUL-terminated buffer. */
char *read_file(const char *path, size_t *len, char *err, size_t err_size);

#endif /* GRADE_COURSE_JSON_H */
