/*
 * grade.h — course grade engine (C11 port of src/grade/engine.ts).
 *
 * Pure computation: no I/O, no global state. Every function is reentrant.
 * Results match the TypeScript engine bit for bit when compiled without
 * floating-point contraction or fast-math (see the Makefile).
 *
 * Model
 *   - An item's raw result is score / out_of. `worth` (defaulting to out_of)
 *     is how much it counts inside its category; out_of == 0 makes the item
 *     pure extra credit.
 *   - A category may drop its lowest/highest results and has a weight that is
 *     used by the weighted scheme.
 *   - A course combines categories by total points or by renormalised weights.
 */
#ifndef GRADE_H
#define GRADE_H

#include <stdbool.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ---------------------------------------------------------------- model --- */

typedef enum {
    /* Sum of points earned / sum of points possible across all categories. */
    GRADE_SCHEME_POINTS,
    /* Each category's percentage times its weight. Categories with nothing
     * graded are left out and the remaining weights renormalised. */
    GRADE_SCHEME_WEIGHTED
} grade_scheme;

typedef struct {
    char *id;           /* optional, may be NULL */
    char *name;
    bool graded;        /* false: score is null (not graded yet) */
    double score;       /* meaningful only when graded */
    double out_of;      /* raw points possible; 0 = extra credit */
    bool has_worth;
    double worth;       /* used when has_worth, otherwise out_of */
    bool excluded;      /* shown but never counted */
    bool never_drop;    /* exempt from drop rules */
    char *note;         /* optional, may be NULL */
} grade_item;

typedef struct {
    char *id;           /* optional, may be NULL */
    char *name;
    double weight;      /* used by GRADE_SCHEME_WEIGHTED only */
    size_t drop_lowest;
    size_t drop_highest;
    bool extra_credit;  /* weighted: adds to the grade, not to the denominator */
    grade_item *items;
    size_t item_count;
} grade_category;

typedef struct {
    char *letter;
    double min;         /* minimum percentage (0-100) for this letter */
} grade_cutoff;

typedef struct {
    char *id;           /* optional, may be NULL */
    char *name;
    grade_scheme scheme;
    grade_category *categories;
    size_t category_count;
    grade_cutoff *scale; /* any order */
    size_t scale_count;
    int display_decimals; /* decimals the course's gradebook shows (default 2) */
    char *notes;        /* optional, may be NULL */
} grade_course;

/* Frees every heap member of `course` (strings and arrays, all of which must
 * have come from malloc) and zeroes it. The struct itself is not freed. */
void grade_course_free(grade_course *course);

/* -------------------------------------------------------------- results --- */

typedef struct {
    const grade_item *item;
    bool counted;       /* graded, not excluded and not dropped */
    bool dropped;
    double earned;      /* score / out_of * worth (raw score for extra credit) */
    double possible;
} grade_item_result;

typedef struct {
    const grade_category *category;
    grade_item_result *items; /* parallel to category->items */
    size_t item_count;
    double earned;
    double possible;
    bool has_percent;   /* false when nothing counted */
    double percent;
} grade_category_result;

typedef struct {
    const grade_course *course;
    grade_category_result *categories; /* parallel to course->categories */
    size_t category_count;
    double earned;      /* points scheme: points; weighted: weight x fraction */
    double possible;    /* points scheme: points; weighted: graded weight */
    bool has_percent;
    double percent;
    const char *letter; /* points into course->scale, or NULL */
} grade_course_result;

typedef enum {
    GRADE_NEED_NO_REMAINING, /* no ungraded items left */
    GRADE_NEED_SECURED,      /* reached even with 0% on everything left */
    GRADE_NEED_IMPOSSIBLE,   /* not reachable even with 100%; value = best % */
    GRADE_NEED_AVERAGE       /* value = average fraction (0-1) needed */
} grade_need_kind;

typedef struct {
    grade_need_kind kind;
    double value;
} grade_needed;

/* ------------------------------------------------------------ functions --- */

/* Effective worth of an item inside its category. */
double grade_item_worth(const grade_item *item);

/* Computes one category, applying drop rules. Returns 0, or -1 if memory
 * allocation failed. On success free `out` with grade_category_result_free. */
int grade_compute_category(const grade_category *category, grade_category_result *out);
void grade_category_result_free(grade_category_result *result);

/* Computes a whole course. Returns 0, or -1 if memory allocation failed.
 * `out` borrows from `course`, which must outlive it.
 * On success free `out` with grade_course_result_free. */
int grade_compute_course(const grade_course *course, grade_course_result *out);
void grade_course_result_free(grade_course_result *result);

/* Letter for a percentage, or NULL when there is no percentage or no scale. */
const char *grade_letter_for(bool has_percent, double percent,
                             const grade_cutoff *scale, size_t scale_count);

/* Number of ungraded, non-excluded items that still have points available. */
size_t grade_remaining_count(const grade_course *course);

/* Average score (as a fraction of points) needed on every remaining item to
 * finish at or above `target` percent. Returns 0, or -1 on allocation failure. */
int grade_needed_on_remaining(const grade_course *course, double target, grade_needed *out);

/* Rounds half away from zero after removing binary noise exactly the way
 * JavaScript's Number(x.toFixed(10)) does, so 52.549999999 rounds like 52.55. */
double grade_round(double x, int decimals);

#ifdef __cplusplus
}
#endif

#endif /* GRADE_H */
