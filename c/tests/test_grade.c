/*
 * Tests for the C grade engine.
 *
 *   build/test_grade EXAMPLES_DIR
 *
 * The course checks use the shared fixtures in c/examples (exported from
 * src/grade/courses.ts) and assert the same figures as src/grade/courses.test.ts,
 * i.e. the numbers printed in docs/<course>/grades-11-5-2026.pdf. The engine
 * checks mirror src/grade/engine.test.ts.
 */
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "course_json.h"
#include "grade.h"
#include "json.h"

static int failures = 0;
static int checks = 0;
static const char *current = "";

#define CHECK(cond)                                                                           \
    do {                                                                                      \
        checks++;                                                                             \
        if (!(cond)) {                                                                        \
            failures++;                                                                       \
            fprintf(stderr, "FAIL [%s] %s:%d: %s\n", current, __FILE__, __LINE__, #cond);     \
        }                                                                                     \
    } while (0)

/* Rounded value equals the figure shown in the gradebook. */
#define CHECK_ROUND(x, decimals, expected)                                                    \
    do {                                                                                      \
        checks++;                                                                             \
        double got_ = grade_round((x), (decimals));                                           \
        if (got_ != (expected)) {                                                             \
            failures++;                                                                       \
            fprintf(stderr, "FAIL [%s] %s:%d: round(%s, %d) = %.17g, expected %.17g\n",       \
                    current, __FILE__, __LINE__, #x, (decimals), got_, (double)(expected));   \
        }                                                                                     \
    } while (0)

/* ------------------------------------------------------------- helpers --- */

static const char *examples_dir = "examples";

static void load(const char *name, grade_course *course)
{
    char path[512], err[512];
    snprintf(path, sizeof path, "%s/%s.json", examples_dir, name);
    size_t len;
    char *text = read_file(path, &len, err, sizeof err);
    if (!text) {
        fprintf(stderr, "cannot read %s: %s\n", path, err);
        exit(2);
    }
    json_value root;
    if (!json_parse(text, len, &root, err, sizeof err) || !course_from_json(&root, course, err, sizeof err)) {
        fprintf(stderr, "%s: %s\n", path, err);
        exit(2);
    }
    json_free(&root);
    free(text);
}

static const grade_category_result *category(const grade_course_result *r, const char *name)
{
    for (size_t i = 0; i < r->category_count; i++)
        if (strcmp(r->categories[i].category->name, name) == 0)
            return &r->categories[i];
    fprintf(stderr, "no category %s\n", name);
    exit(2);
}

static grade_item *item_named(grade_course *course, const char *name)
{
    for (size_t c = 0; c < course->category_count; c++)
        for (size_t i = 0; i < course->categories[c].item_count; i++)
            if (strcmp(course->categories[c].items[i].name, name) == 0)
                return &course->categories[c].items[i];
    fprintf(stderr, "no item %s\n", name);
    exit(2);
}

static bool was_dropped(const grade_course_result *r, const char *name)
{
    for (size_t c = 0; c < r->category_count; c++)
        for (size_t i = 0; i < r->categories[c].item_count; i++)
            if (strcmp(r->categories[c].items[i].item->name, name) == 0)
                return r->categories[c].items[i].dropped;
    fprintf(stderr, "no item %s\n", name);
    exit(2);
}

/* Builders for hand-made test data. */
static grade_item graded(double score, double out_of)
{
    grade_item it = { 0 };
    it.name = "item";
    it.graded = true;
    it.score = score;
    it.out_of = out_of;
    return it;
}

static grade_item ungraded(double out_of)
{
    grade_item it = { 0 };
    it.name = "item";
    it.out_of = out_of;
    return it;
}

static grade_category make_category(grade_item *items, size_t n, double weight)
{
    grade_category c = { 0 };
    c.name = "cat";
    c.weight = weight;
    c.items = items;
    c.item_count = n;
    return c;
}

static grade_cutoff standard_scale[] = {
    { "A", 90 }, { "B", 80 }, { "C", 70 }, { "D", 60 }, { "F", 0 },
};

static grade_course make_course(grade_scheme scheme, grade_category *cats, size_t n)
{
    grade_course c = { 0 };
    c.name = "course";
    c.scheme = scheme;
    c.categories = cats;
    c.category_count = n;
    c.scale = standard_scale;
    c.scale_count = 5;
    c.display_decimals = 2;
    return c;
}

#define COUNT(a) (sizeof(a) / sizeof((a)[0]))

/* --------------------------------------------------- PDF reproductions --- */

static void test_cs401(void)
{
    current = "CS 401 (Canvas total points)";
    grade_course course;
    grade_course_result r;
    load("cs401", &course);
    CHECK(grade_compute_course(&course, &r) == 0);
    /* Total 56.95% (106.50 / 187.00) */
    CHECK_ROUND(r.percent, 2, 56.95);
    CHECK_ROUND(r.earned, 2, 106.5);
    CHECK_ROUND(r.possible, 2, 187);
    const grade_category_result *c = category(&r, "Homework");
    CHECK_ROUND(c->percent, 2, 97.92);
    CHECK_ROUND(c->earned, 2, 70.5);
    CHECK_ROUND(c->possible, 2, 72);
    c = category(&r, "Check-Ins");
    CHECK_ROUND(c->percent, 2, 66.67);
    CHECK_ROUND(c->earned, 2, 10);
    CHECK_ROUND(c->possible, 2, 15);
    c = category(&r, "Exams");
    CHECK_ROUND(c->percent, 2, 26);
    /* Late Days does not count: Canvas shows "N/A 0.00 / 0.00". */
    c = category(&r, "Assignments");
    CHECK(!c->has_percent && c->earned == 0 && c->possible == 0);
    grade_course_result_free(&r);
    grade_course_free(&course);
}

static void test_cs411(void)
{
    current = "CS 411 (Canvas weighted, drop lowest 2)";
    grade_course course;
    grade_course_result r;
    load("cs411", &course);
    CHECK(grade_compute_course(&course, &r) == 0);
    CHECK_ROUND(r.percent, 2, 79.53);
    const grade_category_result *c = category(&r, "Extra Credit Assignments");
    CHECK_ROUND(c->percent, 2, 76.67);
    CHECK_ROUND(c->earned, 2, 6.9);
    CHECK_ROUND(c->possible, 2, 9);
    c = category(&r, "Quiz");
    CHECK_ROUND(c->percent, 2, 93.33);
    CHECK_ROUND(c->earned, 2, 28);
    CHECK_ROUND(c->possible, 2, 30);
    c = category(&r, "Projects");
    CHECK_ROUND(c->percent, 2, 82.25);
    CHECK_ROUND(c->earned, 2, 164.5);
    c = category(&r, "Labs");
    CHECK_ROUND(c->percent, 2, 95);
    CHECK_ROUND(c->earned, 2, 57);
    CHECK_ROUND(c->possible, 2, 60);
    c = category(&r, "Exams");
    CHECK_ROUND(c->percent, 2, 51);
    CHECK(!category(&r, "Final Exam")->has_percent);
    CHECK(!category(&r, "Make-up Project/Exam")->has_percent);
    CHECK(was_dropped(&r, "Lab07") && was_dropped(&r, "Lab08") && was_dropped(&r, "Quiz 03"));
    /* Quiz 04 and Quiz 05 are both 8/10: exactly one of them is dropped. */
    CHECK(was_dropped(&r, "Quiz 04") != was_dropped(&r, "Quiz 05"));
    CHECK(r.letter && strcmp(r.letter, "C") == 0);
    grade_course_result_free(&r);
    grade_course_free(&course);
}

static void test_cs440(void)
{
    current = "CS 440 (Blackboard scaled points)";
    grade_course course;
    grade_course_result r;
    load("cs440", &course);
    CHECK(grade_compute_course(&course, &r) == 0);
    /* Total (out of 67) 52.6, PPP 78.4 */
    CHECK_ROUND(r.possible, 1, 67);
    CHECK_ROUND(r.earned, 1, 52.6);
    CHECK_ROUND(r.percent, 1, 78.4);
    grade_course_result_free(&r);
    grade_course_free(&course);
}

static void test_math218(void)
{
    current = "Math 218 (Canvas setup)";
    grade_course course;
    grade_course_result r;
    load("math218-canvas", &course);
    CHECK(grade_compute_course(&course, &r) == 0);
    /* Total 118.28% (241.29 / 204.00) */
    CHECK_ROUND(r.percent, 2, 118.28);
    CHECK_ROUND(r.earned, 2, 241.29);
    CHECK_ROUND(r.possible, 2, 204);
    const grade_category_result *c = category(&r, "Written Homework group");
    CHECK_ROUND(c->percent, 2, 96.3);
    CHECK_ROUND(c->earned, 2, 52);
    c = category(&r, "MOM Homework group");
    CHECK_ROUND(c->percent, 2, 97.9);
    c = category(&r, "Midterm Exams");
    CHECK_ROUND(c->percent, 2, 52);
    CHECK(!category(&r, "Assignments")->has_percent);
    /* "Midsemester Score 65.39 / 0" -> "N/A 65.39 / 0.00" */
    c = category(&r, "Midsemester Grade");
    CHECK(!c->has_percent);
    CHECK_ROUND(c->earned, 2, 65.39);
    grade_course_result_free(&r);
    grade_course_free(&course);

    current = "Math 218 (syllabus, midsemester snapshot)";
    load("math218", &course);
    /* At mid-semester MOM HW 1-3 and Written HW 1-2 were graded, no drops yet. */
    for (size_t i = 0; i < course.category_count; i++)
        course.categories[i].drop_lowest = 0;
    item_named(&course, "Homework 4 (Weeks 7-8)")->graded = false;
    item_named(&course, "Homework 5 (Weeks 9-10)")->graded = false;
    item_named(&course, "Written Homework #3")->graded = false;
    CHECK(grade_compute_course(&course, &r) == 0);
    CHECK_ROUND(r.percent, 2, 65.39);
    grade_course_result_free(&r);
    grade_course_free(&course);

    current = "Math 218 (syllabus, today)";
    load("math218", &course);
    CHECK(grade_compute_course(&course, &r) == 0);
    CHECK_ROUND(category(&r, "MOM Homework (online)")->percent, 2, 99.13);
    CHECK_ROUND(category(&r, "Written Homework")->percent, 2, 100);
    CHECK_ROUND(category(&r, "Exams")->percent, 2, 52);
    CHECK_ROUND(r.percent, 2, grade_round(15 * 0.99125 + 15 * 1 + 70 * 0.52, 2));
    CHECK(r.letter && strcmp(r.letter, "C") == 0);
    grade_course_result_free(&r);
    grade_course_free(&course);
}

/* -------------------------------------------------------- engine rules --- */

static void test_drops(void)
{
    grade_category_result r;

    current = "drop lowest maximises the result";
    /* Dropping the 0/1 gives 40/110 = 36%; dropping the 30/100 gives 10/11 = 91%. */
    grade_item a[] = { graded(0, 1), graded(30, 100), graded(10, 10) };
    grade_category ca = make_category(a, COUNT(a), 0);
    ca.drop_lowest = 1;
    CHECK(grade_compute_category(&ca, &r) == 0);
    CHECK(!r.items[0].dropped && r.items[1].dropped && !r.items[2].dropped);
    CHECK_ROUND(r.percent, 2, 90.91);
    grade_category_result_free(&r);

    current = "drop highest";
    grade_item b[] = { graded(10, 10), graded(5, 10), graded(7, 10) };
    grade_category cb = make_category(b, COUNT(b), 0);
    cb.drop_highest = 1;
    CHECK(grade_compute_category(&cb, &r) == 0);
    CHECK(r.items[0].dropped && !r.items[1].dropped && !r.items[2].dropped);
    CHECK(r.percent == 60);
    grade_category_result_free(&r);

    current = "never drop, and keep at least one";
    grade_item c[] = { graded(0, 10), graded(5, 10) };
    c[0].never_drop = true;
    grade_category cc = make_category(c, COUNT(c), 0);
    cc.drop_lowest = 1;
    CHECK(grade_compute_category(&cc, &r) == 0);
    CHECK(!r.items[0].dropped && r.items[1].dropped);
    grade_category_result_free(&r);
    grade_item d[] = { graded(2, 10) };
    grade_category cd = make_category(d, COUNT(d), 0);
    cd.drop_lowest = 3;
    CHECK(grade_compute_category(&cd, &r) == 0);
    CHECK(r.percent == 20);
    grade_category_result_free(&r);

    current = "ungraded and excluded items are not drop candidates";
    grade_item e[] = { ungraded(10), graded(1, 10), graded(4, 10), graded(8, 10) };
    e[1].excluded = true;
    grade_category ce = make_category(e, COUNT(e), 0);
    ce.drop_lowest = 1;
    CHECK(grade_compute_category(&ce, &r) == 0);
    CHECK(r.percent == 80);
    grade_category_result_free(&r);
}

static void test_worth_and_extra_credit(void)
{
    grade_category_result r;
    current = "worth scales the raw score";
    grade_item a[] = { graded(85, 100) };
    a[0].has_worth = true;
    a[0].worth = 10;
    grade_category ca = make_category(a, COUNT(a), 0);
    CHECK(grade_compute_category(&ca, &r) == 0);
    CHECK(fabs(r.earned - 8.5) < 1e-12 && r.possible == 10);
    grade_category_result_free(&r);

    current = "zero-point items are extra credit";
    grade_item b[] = { graded(9, 10), graded(2, 0) };
    grade_category cb = make_category(b, COUNT(b), 0);
    CHECK(grade_compute_category(&cb, &r) == 0);
    CHECK(r.earned == 11 && r.possible == 10);
    grade_category_result_free(&r);
}

static void test_weighted(void)
{
    grade_course_result r;
    current = "weighted renormalises over graded categories";
    grade_item a[] = { graded(9, 10) }, b[] = { graded(70, 100) }, c[] = { ungraded(100) };
    grade_category cats[] = { make_category(a, 1, 30), make_category(b, 1, 20), make_category(c, 1, 50) };
    grade_course course = make_course(GRADE_SCHEME_WEIGHTED, cats, COUNT(cats));
    CHECK(grade_compute_course(&course, &r) == 0);
    CHECK_ROUND(r.percent, 4, grade_round(((30 * 0.9 + 20 * 0.7) / 50) * 100, 4));
    grade_course_result_free(&r);

    current = "extra-credit category adds without raising the denominator";
    grade_item d[] = { graded(80, 100) }, e[] = { graded(1, 1) };
    grade_category cats2[] = { make_category(d, 1, 100), make_category(e, 1, 5) };
    cats2[1].extra_credit = true;
    grade_course course2 = make_course(GRADE_SCHEME_WEIGHTED, cats2, COUNT(cats2));
    CHECK(grade_compute_course(&course2, &r) == 0);
    CHECK(fabs(r.percent - 85) < 1e-9);
    grade_course_result_free(&r);
}

static void test_letters_and_needed(void)
{
    current = "letters";
    CHECK(strcmp(grade_letter_for(true, 89.99, standard_scale, 5), "B") == 0);
    CHECK(strcmp(grade_letter_for(true, 90, standard_scale, 5), "A") == 0);
    CHECK(strcmp(grade_letter_for(true, 89.9999999999, standard_scale, 5), "A") == 0);
    CHECK(grade_letter_for(false, 0, standard_scale, 5) == NULL);
    grade_cutoff shuffled[] = { { "C", 70 }, { "A", 90 }, { "F", 0 }, { "B", 80 } };
    CHECK(strcmp(grade_letter_for(true, 85, shuffled, 4), "B") == 0);
    CHECK(strcmp(grade_letter_for(true, -5, shuffled, 4), "F") == 0);

    current = "needed on remaining";
    /* 60/100 so far, 100 points left; 80% overall needs 100/100. */
    grade_item a[] = { graded(60, 100), ungraded(100) };
    grade_category cats[] = { make_category(a, COUNT(a), 0) };
    grade_course course = make_course(GRADE_SCHEME_POINTS, cats, 1);
    grade_needed n;
    CHECK(grade_needed_on_remaining(&course, 80, &n) == 0);
    CHECK(n.kind == GRADE_NEED_AVERAGE && fabs(n.value - 1) < 1e-6);
    CHECK(grade_needed_on_remaining(&course, 70, &n) == 0 && n.kind == GRADE_NEED_AVERAGE);
    CHECK(grade_needed_on_remaining(&course, 81, &n) == 0 && n.kind == GRADE_NEED_IMPOSSIBLE);
    CHECK(grade_needed_on_remaining(&course, 30, &n) == 0 && n.kind == GRADE_NEED_SECURED);
    a[1].graded = true;
    CHECK(grade_needed_on_remaining(&course, 80, &n) == 0 && n.kind == GRADE_NEED_NO_REMAINING);
}

static void test_rounding(void)
{
    current = "rounding matches JavaScript";
    CHECK_ROUND(52.55, 1, 52.6);
    CHECK_ROUND(52.549999999999997, 1, 52.6);  /* binary noise removed first */
    CHECK_ROUND(0.00048828125, 10, 0.0004882813); /* toFixed tie goes up */
    CHECK_ROUND(-0.00048828125, 10, -0.0004882813);
    CHECK_ROUND(2.5, 0, 3);
    CHECK_ROUND(-2.5, 0, -3);
    CHECK_ROUND(9.99999999999, 2, 10);
    CHECK_ROUND(56.951871657754, 2, 56.95);
    CHECK(isnan(grade_round(NAN, 2)));
}

int main(int argc, char **argv)
{
    if (argc > 1)
        examples_dir = argv[1];
    test_cs401();
    test_cs411();
    test_cs440();
    test_math218();
    test_drops();
    test_worth_and_extra_credit();
    test_weighted();
    test_letters_and_needed();
    test_rounding();
    if (failures) {
        fprintf(stderr, "%d of %d checks failed\n", failures, checks);
        return 1;
    }
    printf("all %d checks passed\n", checks);
    return 0;
}
