/*
 * grade.c — course grade engine (C11 port of src/grade/engine.ts).
 *
 * The order of floating-point operations deliberately mirrors the TypeScript
 * engine so that both produce identical doubles. Do not "simplify" sums or
 * products without checking c/tools/difftest.mjs afterwards.
 */
#include "grade.h"

#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* An optional fill-in score for ungraded items, used by the needed-score
 * solver (the TypeScript engine's withRemainingAt). */
typedef struct {
    bool active;
    double fraction;
} fill_rule;

static const fill_rule NO_FILL = { false, 0.0 };

double grade_item_worth(const grade_item *item)
{
    return item->has_worth ? item->worth : item->out_of;
}

/* Score an item counts with, or false when it has none. */
static bool item_score(const grade_item *item, const fill_rule *fill, double *score)
{
    if (item->graded) {
        *score = item->score;
        return true;
    }
    if (fill->active && !item->excluded && item->out_of > 0) {
        *score = item->out_of * fill->fraction;
        return true;
    }
    return false;
}

static void contribution(const grade_item *item, double score, double *earned, double *possible)
{
    double worth = grade_item_worth(item);
    if (item->out_of > 0) {
        *earned = (score / item->out_of) * worth;
        *possible = worth;
    } else {
        /* Zero-point items are pure extra credit: the raw score is added on top. */
        *earned = score;
        *possible = 0;
    }
}

/* ------------------------------------------------------------ drop rules --- */

typedef struct {
    size_t index; /* item index inside the category */
    double earned;
    double possible;
} candidate;

typedef struct {
    double v;
    size_t index;
    size_t pos; /* position in the candidate array */
} scored;

/* Descending by value; ties keep the earlier item first (stable, total order). */
static int compare_scored(const void *a, const void *b)
{
    const scored *x = a;
    const scored *y = b;
    if (x->v > y->v)
        return -1;
    if (x->v < y->v)
        return 1;
    return (x->index > y->index) - (x->index < y->index);
}

/* Sorts candidates by sign * (earned - q * possible) into `s` and returns the
 * sum of the best `keep` values. */
static double pick(const candidate *c, size_t n, size_t keep, double sign, double q, scored *s)
{
    for (size_t i = 0; i < n; i++) {
        s[i].v = sign * (c[i].earned - q * c[i].possible);
        s[i].index = c[i].index;
        s[i].pos = i;
    }
    qsort(s, n, sizeof *s, compare_scored);
    double sum = 0;
    for (size_t i = 0; i < keep; i++)
        sum += s[i].v;
    return sum;
}

/*
 * Chooses which `keep` candidates to keep so that
 *     (fixed_earned + sum earned) / (fixed_possible + sum possible)
 * is maximised (maximise = true, dropping the lowest) or minimised (dropping
 * the highest). With mixed point values "drop the lowest percentage" is not
 * optimal, so like Canvas we search on the resulting ratio q: q is reachable
 * iff the best `keep` values of (earned - q * possible) sum to at least
 * -(fixed_earned - q * fixed_possible). Bisecting on q finds the optimum.
 *
 * Writes kept[i] for each candidate. Returns 0, or -1 on allocation failure.
 */
static int choose_kept(const candidate *c, size_t n, size_t keep, double fixed_earned,
                       double fixed_possible, bool maximise, bool *kept)
{
    if (keep >= n) {
        for (size_t i = 0; i < n; i++)
            kept[i] = true;
        return 0;
    }
    for (size_t i = 0; i < n; i++)
        kept[i] = false;
    if (keep == 0)
        return 0;

    scored *s = malloc(n * sizeof *s);
    if (!s)
        return -1;
    double sign = maximise ? 1.0 : -1.0;

    double min_positive = INFINITY;
    if (fixed_possible > 0)
        min_positive = fixed_possible;
    for (size_t i = 0; i < n; i++)
        if (c[i].possible > 0 && c[i].possible < min_positive)
            min_positive = c[i].possible;

    double q = 0;
    if (isfinite(min_positive)) {
        double positive_earned = 0;
        for (size_t i = 0; i < n; i++)
            positive_earned += c[i].earned > 0 ? c[i].earned : 0;
        double total_earned = fixed_earned + positive_earned;
        double lo = 0;
        double hi = total_earned / min_positive + 1;
        for (int iter = 0; iter < 200; iter++) {
            double mid = (lo + hi) / 2;
            double fixed_part = sign * (fixed_earned - mid * fixed_possible);
            bool feasible = fixed_part + pick(c, n, keep, sign, mid, s) >= 0;
            /* maximise: largest reachable q; minimise: smallest reachable q */
            if (feasible == maximise)
                lo = mid;
            else
                hi = mid;
        }
        q = maximise ? lo : hi;
    }
    /* Without any denominator (only extra credit) q stays 0: keep the largest
     * (or smallest) raw scores. */
    pick(c, n, keep, sign, q, s);
    for (size_t i = 0; i < keep; i++)
        kept[s[i].pos] = true;
    free(s);
    return 0;
}

/* Keeps only the candidates marked in kept[], preserving order. */
static size_t compact(candidate *c, size_t n, const bool *kept)
{
    size_t m = 0;
    for (size_t i = 0; i < n; i++)
        if (kept[i])
            c[m++] = c[i];
    return m;
}

static int apply_drops(const grade_category *category, grade_item_result *items, size_t n)
{
    candidate *pool = malloc(n * sizeof *pool);
    bool *kept = malloc(n * sizeof *kept);
    bool *in_pool = calloc(n, sizeof *in_pool);
    int rc = -1;
    if (!pool || !kept || !in_pool)
        goto out;

    double fixed_earned = 0;
    double fixed_possible = 0;
    size_t fixed_count = 0;
    size_t m = 0;
    for (size_t i = 0; i < n; i++) {
        if (!items[i].counted)
            continue;
        if (items[i].item->never_drop) {
            fixed_earned += items[i].earned;
            fixed_possible += items[i].possible;
            fixed_count++;
        } else {
            pool[m].index = i;
            pool[m].earned = items[i].earned;
            pool[m].possible = items[i].possible;
            m++;
        }
    }

    /* Like Canvas, never drop every item: at least one stays. */
    size_t min_keep = fixed_count > 0 ? 0 : 1;

    size_t available = m > min_keep ? m - min_keep : 0;
    size_t low = category->drop_lowest < available ? category->drop_lowest : available;
    if (low > 0) {
        if (choose_kept(pool, m, m - low, fixed_earned, fixed_possible, true, kept) != 0)
            goto out;
        m = compact(pool, m, kept);
    }
    available = m > min_keep ? m - min_keep : 0;
    size_t high = category->drop_highest < available ? category->drop_highest : available;
    if (high > 0) {
        if (choose_kept(pool, m, m - high, fixed_earned, fixed_possible, false, kept) != 0)
            goto out;
        m = compact(pool, m, kept);
    }

    for (size_t i = 0; i < m; i++)
        in_pool[pool[i].index] = true;
    for (size_t i = 0; i < n; i++) {
        if (items[i].counted && !items[i].item->never_drop && !in_pool[i]) {
            items[i].counted = false;
            items[i].dropped = true;
        }
    }
    rc = 0;
out:
    free(pool);
    free(kept);
    free(in_pool);
    return rc;
}

/* ------------------------------------------------------------ categories --- */

static int compute_category_impl(const grade_category *category, const fill_rule *fill,
                                 grade_category_result *out)
{
    memset(out, 0, sizeof *out);
    out->category = category;
    size_t n = category->item_count;
    if (n > 0) {
        out->items = calloc(n, sizeof *out->items);
        if (!out->items)
            return -1;
    }
    out->item_count = n;

    for (size_t i = 0; i < n; i++) {
        const grade_item *item = &category->items[i];
        grade_item_result *r = &out->items[i];
        double score;
        bool graded = item_score(item, fill, &score) && !item->excluded;
        r->item = item;
        r->counted = graded;
        r->dropped = false;
        if (graded)
            contribution(item, score, &r->earned, &r->possible);
    }

    if ((category->drop_lowest > 0 || category->drop_highest > 0) && n > 0) {
        if (apply_drops(category, out->items, n) != 0) {
            grade_category_result_free(out);
            return -1;
        }
    }

    double earned = 0;
    double possible = 0;
    for (size_t i = 0; i < n; i++) {
        if (out->items[i].counted) {
            earned += out->items[i].earned;
            possible += out->items[i].possible;
        }
    }
    out->earned = earned;
    out->possible = possible;
    out->has_percent = possible > 0;
    out->percent = possible > 0 ? (earned / possible) * 100 : 0;
    return 0;
}

int grade_compute_category(const grade_category *category, grade_category_result *out)
{
    return compute_category_impl(category, &NO_FILL, out);
}

void grade_category_result_free(grade_category_result *result)
{
    if (!result)
        return;
    free(result->items);
    result->items = NULL;
    result->item_count = 0;
}

/* --------------------------------------------------------------- letters --- */

const char *grade_letter_for(bool has_percent, double percent, const grade_cutoff *scale,
                             size_t scale_count)
{
    if (!has_percent || scale_count == 0)
        return NULL;
    /* Highest cutoff first; ties keep their original order. Done without
     * allocating: repeatedly take the highest cutoff not yet visited. */
    const grade_cutoff *last = NULL;
    double bound = INFINITY;
    size_t bound_index = 0;
    for (size_t visited = 0; visited < scale_count; visited++) {
        const grade_cutoff *best = NULL;
        size_t best_index = 0;
        for (size_t i = 0; i < scale_count; i++) {
            double m = scale[i].min;
            /* Next in (min descending, index ascending) order after `bound`. */
            bool after = visited == 0 || m < bound || (m == bound && i > bound_index);
            if (!after)
                continue;
            if (!best || m > best->min || (m == best->min && i < best_index)) {
                best = &scale[i];
                best_index = i;
            }
        }
        if (!best)
            break;
        /* Small tolerance so 89.99999999 from floating point earns the 90 cutoff. */
        if (percent + 1e-9 >= best->min)
            return best->letter;
        last = best;
        bound = best->min;
        bound_index = best_index;
    }
    return last ? last->letter : NULL;
}

/* --------------------------------------------------------------- courses --- */

static int compute_course_impl(const grade_course *course, const fill_rule *fill,
                               grade_course_result *out)
{
    memset(out, 0, sizeof *out);
    out->course = course;
    size_t n = course->category_count;
    if (n > 0) {
        out->categories = calloc(n, sizeof *out->categories);
        if (!out->categories)
            return -1;
    }
    for (size_t i = 0; i < n; i++) {
        if (compute_category_impl(&course->categories[i], fill, &out->categories[i]) != 0) {
            out->category_count = i;
            grade_course_result_free(out);
            return -1;
        }
    }
    out->category_count = n;

    double earned = 0;
    double possible = 0;
    if (course->scheme == GRADE_SCHEME_POINTS) {
        for (size_t i = 0; i < n; i++) {
            earned += out->categories[i].earned;
            possible += out->categories[i].possible;
        }
    } else {
        for (size_t i = 0; i < n; i++) {
            const grade_category_result *c = &out->categories[i];
            double w = c->category->weight;
            if (!(w > 0) || !c->has_percent)
                continue;
            earned += (w * c->percent) / 100;
            if (!c->category->extra_credit)
                possible += w;
        }
    }
    out->earned = earned;
    out->possible = possible;
    out->has_percent = possible > 0;
    out->percent = possible > 0 ? (earned / possible) * 100 : 0;
    out->letter = grade_letter_for(out->has_percent, out->percent, course->scale, course->scale_count);
    return 0;
}

int grade_compute_course(const grade_course *course, grade_course_result *out)
{
    return compute_course_impl(course, &NO_FILL, out);
}

void grade_course_result_free(grade_course_result *result)
{
    if (!result)
        return;
    for (size_t i = 0; i < result->category_count; i++)
        grade_category_result_free(&result->categories[i]);
    free(result->categories);
    result->categories = NULL;
    result->category_count = 0;
}

/* -------------------------------------------------------- needed scores --- */

size_t grade_remaining_count(const grade_course *course)
{
    size_t count = 0;
    for (size_t c = 0; c < course->category_count; c++) {
        const grade_category *cat = &course->categories[c];
        for (size_t i = 0; i < cat->item_count; i++) {
            const grade_item *it = &cat->items[i];
            if (!it->graded && !it->excluded && it->out_of > 0)
                count++;
        }
    }
    return count;
}

/* Course percentage with every remaining item at `fraction` of its points. */
static int percent_at(const grade_course *course, double fraction, double *percent)
{
    fill_rule fill = { true, fraction };
    grade_course_result r;
    if (compute_course_impl(course, &fill, &r) != 0)
        return -1;
    *percent = r.has_percent ? r.percent : 0;
    grade_course_result_free(&r);
    return 0;
}

int grade_needed_on_remaining(const grade_course *course, double target, grade_needed *out)
{
    out->value = 0;
    if (grade_remaining_count(course) == 0) {
        out->kind = GRADE_NEED_NO_REMAINING;
        return 0;
    }
    double p;
    if (percent_at(course, 0, &p) != 0)
        return -1;
    if (p + 1e-9 >= target) {
        out->kind = GRADE_NEED_SECURED;
        return 0;
    }
    if (percent_at(course, 1, &p) != 0)
        return -1;
    if (p + 1e-9 < target) {
        out->kind = GRADE_NEED_IMPOSSIBLE;
        out->value = p;
        return 0;
    }
    double lo = 0;
    double hi = 1;
    for (int i = 0; i < 60; i++) {
        double mid = (lo + hi) / 2;
        if (percent_at(course, mid, &p) != 0)
            return -1;
        if (p + 1e-9 >= target)
            hi = mid;
        else
            lo = mid;
    }
    out->kind = GRADE_NEED_AVERAGE;
    out->value = hi;
    return 0;
}

/* -------------------------------------------------------------- rounding --- */

/*
 * Number(x.toFixed(10)) from JavaScript: round the exact decimal value of x to
 * 10 places, ties away from zero. printf's own rounding at a tie depends on
 * the libc (glibc rounds to even), so the exact digits are printed and the
 * rounding is done by hand.
 */
static double js_to_fixed_10(double x)
{
    if (!isfinite(x) || fabs(x) >= 1e21)
        return x; /* toFixed falls back to String(x) */
    /* 1080 fraction digits print every finite double exactly. */
    char buf[1120];
    int len = snprintf(buf, sizeof buf, "%.1080f", fabs(x));
    if (len < 0 || (size_t)len >= sizeof buf)
        return x;
    char *dot = strchr(buf, '.');
    if (!dot)
        return x;
    bool round_up = dot[11] >= '5';
    dot[11] = '\0';
    if (round_up) {
        char *p = dot + 10;
        for (;;) {
            if (p == dot) {
                p--;
                continue;
            }
            if (*p != '9') {
                (*p)++;
                break;
            }
            *p = '0';
            if (p == buf) {
                /* Carry out of the leading digit: 9.99... -> 10.00... */
                memmove(buf + 1, buf, strlen(buf) + 1);
                buf[0] = '1';
                break;
            }
            p--;
        }
    }
    double v = strtod(buf, NULL);
    return x < 0 ? -v : v;
}

double grade_round(double x, int decimals)
{
    double f = pow(10.0, decimals);
    double clean = js_to_fixed_10(x);
    /* Math.sign: +-1, or the value itself for +-0 and NaN. */
    double sign = clean > 0 ? 1.0 : (clean < 0 ? -1.0 : clean);
    return (sign * round(fabs(clean) * f)) / f;
}

/* ---------------------------------------------------------------- memory --- */

void grade_course_free(grade_course *course)
{
    if (!course)
        return;
    for (size_t c = 0; c < course->category_count; c++) {
        grade_category *cat = &course->categories[c];
        for (size_t i = 0; i < cat->item_count; i++) {
            free(cat->items[i].id);
            free(cat->items[i].name);
            free(cat->items[i].note);
        }
        free(cat->items);
        free(cat->id);
        free(cat->name);
    }
    free(course->categories);
    for (size_t i = 0; i < course->scale_count; i++)
        free(course->scale[i].letter);
    free(course->scale);
    free(course->id);
    free(course->name);
    free(course->notes);
    memset(course, 0, sizeof *course);
}
