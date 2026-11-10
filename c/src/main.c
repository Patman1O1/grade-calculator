/*
 * grade — command-line course grade calculator.
 *
 * Reads course JSON (the format the web app exports: one course object or an
 * array of them) and prints each course's grade, category breakdown and,
 * optionally, the average needed on the remaining work.
 */
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "course_json.h"
#include "grade.h"
#include "json.h"

#define GRADE_VERSION "1.0.0"
#define MAX_TARGETS 32

typedef struct {
    bool json;
    bool summary;
    bool needed;
    double targets[MAX_TARGETS];
    size_t target_count;
} options;

static const char *progname = "grade";

static void usage(FILE *out)
{
    fprintf(out,
            "usage: %s [options] [FILE...]\n"
            "\n"
            "Computes course grades from course JSON: one course object or an array of\n"
            "them, in the format the web app exports. Reads standard input when no FILE\n"
            "is given or FILE is \"-\".\n"
            "\n"
            "options:\n"
            "  -j, --json          print one JSON object per course instead of text\n"
            "  -s, --summary       text output: categories only, without items\n"
            "  -n, --needed        show the average needed on remaining work for each\n"
            "                      letter on the course's scale\n"
            "  -t, --target PCT    show the average needed to finish with at least PCT\n"
            "                      percent (may be repeated)\n"
            "  -h, --help          show this help\n"
            "  -V, --version       show the version\n"
            "\n"
            "exit status: 0 on success, 1 if any input was invalid, 2 on usage errors\n",
            progname);
}

/* ------------------------------------------------------------ formatting --- */

/* round(), and never print "-0". */
static double rounded(double x, int decimals)
{
    double r = grade_round(x, decimals);
    return r == 0 ? 0.0 : r;
}

/* Like the web app's formatNumber: fixed decimals. */
static const char *fmt_fixed(char *buf, size_t size, double x, int decimals)
{
    snprintf(buf, size, "%.*f", decimals, rounded(x, decimals));
    return buf;
}

/* Like the web app's formatPercent: up to `decimals` places, zeros trimmed. */
static const char *fmt_percent(char *buf, size_t size, double x, int decimals)
{
    snprintf(buf, size - 1, "%.*f", decimals, rounded(x, decimals));
    if (strchr(buf, '.')) {
        char *end = buf + strlen(buf) - 1;
        while (*end == '0')
            *end-- = '\0';
        if (*end == '.')
            *end = '\0';
    }
    strcat(buf, "%");
    return buf;
}

/* Shortest plain-decimal form that reads back exactly (like String(x) in JS
 * for ordinary values): 18, 4.2, 65.39. */
static const char *fmt_raw(char *buf, size_t size, double x)
{
    for (int d = 0; d <= 17; d++) {
        snprintf(buf, size, "%.*f", d, x);
        if (strtod(buf, NULL) == x)
            return buf;
    }
    snprintf(buf, size, "%.17g", x);
    return buf;
}

/* Display width of a UTF-8 string, counting code points. */
static int width_of(const char *s)
{
    int w = 0;
    for (; *s; s++)
        if (((unsigned char)*s & 0xC0) != 0x80)
            w++;
    return w;
}

static void pad(const char *s, int width, bool right)
{
    int fill = width - width_of(s);
    if (right)
        printf("%*s%s", fill > 0 ? fill : 0, "", s);
    else
        printf("%s%*s", s, fill > 0 ? fill : 0, "");
}

/* -------------------------------------------------------------- text out --- */

#define NAME_W 40
#define SCORE_W 14
#define COUNT_W 20
#define PCT_W 10

static void describe_need(char *buf, size_t size, const grade_needed *n)
{
    char tmp[64];
    switch (n->kind) {
    case GRADE_NEED_NO_REMAINING:
        snprintf(buf, size, "nothing left to grade");
        break;
    case GRADE_NEED_SECURED:
        snprintf(buf, size, "secured");
        break;
    case GRADE_NEED_IMPOSSIBLE:
        snprintf(buf, size, "out of reach (max %s)", fmt_percent(tmp, sizeof tmp, n->value, 2));
        break;
    case GRADE_NEED_AVERAGE:
        snprintf(buf, size, "%s avg", fmt_percent(tmp, sizeof tmp, n->value * 100, 1));
        break;
    }
}

static void print_text(const grade_course_result *r, const needed_answer *answers,
                       size_t answer_count, const options *opt)
{
    const grade_course *course = r->course;
    const int d = course->display_decimals;
    const bool weighted = course->scheme == GRADE_SCHEME_WEIGHTED;
    char a[64], b[64], line[160];

    printf("%s  [%s]\n\n", course->name, weighted ? "weighted categories" : "total points");

    for (size_t c = 0; c < r->category_count; c++) {
        const grade_category_result *cr = &r->categories[c];
        const grade_category *cat = cr->category;

        /* Category header: name plus its rules. */
        char rules[96] = "";
        size_t used = 0;
        const char *sep = " (";
        if (weighted) {
            used += (size_t)snprintf(rules + used, sizeof rules - used, "%sweight %s%s", sep,
                                     fmt_raw(a, sizeof a, cat->weight), cat->extra_credit ? " EC" : "");
            sep = ", ";
        }
        if (cat->drop_lowest && used < sizeof rules) {
            used += (size_t)snprintf(rules + used, sizeof rules - used, "%sdrop lowest %zu", sep,
                                     cat->drop_lowest);
            sep = ", ";
        }
        if (cat->drop_highest && used < sizeof rules) {
            used += (size_t)snprintf(rules + used, sizeof rules - used, "%sdrop highest %zu", sep,
                                     cat->drop_highest);
            sep = ", ";
        }
        if (sep[0] == ',' && used < sizeof rules)
            snprintf(rules + used, sizeof rules - used, ")");
        snprintf(line, sizeof line, "%s%s", cat->name, rules);

        printf("  ");
        pad(line, NAME_W + 2 + SCORE_W, false);
        snprintf(line, sizeof line, "%s / %s", fmt_fixed(a, sizeof a, cr->earned, d),
                 fmt_fixed(b, sizeof b, cr->possible, d));
        pad(line, COUNT_W, true);
        pad(cr->has_percent ? fmt_percent(a, sizeof a, cr->percent, d) : "N/A", PCT_W, true);
        printf("\n");

        if (opt->summary)
            continue;
        for (size_t i = 0; i < cr->item_count; i++) {
            const grade_item_result *ir = &cr->items[i];
            const grade_item *it = ir->item;
            printf("    ");
            pad(it->name, NAME_W, false);
            snprintf(line, sizeof line, "%s / %s",
                     it->graded ? fmt_raw(a, sizeof a, it->score) : "-",
                     fmt_raw(b, sizeof b, it->out_of));
            pad(line, SCORE_W, true);
            if (it->excluded)
                snprintf(line, sizeof line, "excluded");
            else if (ir->dropped)
                snprintf(line, sizeof line, "dropped");
            else if (!ir->counted)
                snprintf(line, sizeof line, "not graded");
            else
                snprintf(line, sizeof line, "%s / %s", fmt_fixed(a, sizeof a, ir->earned, d),
                         fmt_fixed(b, sizeof b, ir->possible, d));
            pad(line, COUNT_W, true);
            printf("\n");
        }
    }

    printf("\n  Current grade: %s",
           r->has_percent ? fmt_percent(a, sizeof a, r->percent, d) : "N/A");
    if (r->letter)
        printf(" (%s)", r->letter);
    printf("\n");
    if (!weighted) {
        printf("  %s / %s points\n", fmt_fixed(a, sizeof a, r->earned, d),
               fmt_fixed(b, sizeof b, r->possible, d));
    } else if (r->possible > 0) {
        double total = 0;
        for (size_t c = 0; c < course->category_count; c++)
            if (course->categories[c].weight > 0 && !course->categories[c].extra_credit)
                total += course->categories[c].weight;
        printf("  Based on %s of %s category weight graded so far\n",
               fmt_raw(a, sizeof a, r->possible), fmt_raw(b, sizeof b, total));
    }

    if (answer_count > 0) {
        size_t remaining = grade_remaining_count(course);
        printf("\n  Average needed on the %zu remaining item%s to finish with:\n", remaining,
               remaining == 1 ? "" : "s");
        for (size_t i = 0; i < answer_count; i++) {
            char label[96];
            if (answers[i].label)
                snprintf(label, sizeof label, "%s  >= %s", answers[i].label,
                         fmt_percent(a, sizeof a, answers[i].target, 2));
            else
                snprintf(label, sizeof label, "%s", fmt_percent(a, sizeof a, answers[i].target, 2));
            describe_need(line, sizeof line, &answers[i].needed);
            printf("    ");
            pad(label, 18, false);
            printf("%s\n", line);
        }
    }
}

/* ------------------------------------------------------------ processing --- */

/* Letters with a positive cutoff, highest first (ties keep scale order). */
static size_t letter_targets(const grade_course *course, const grade_cutoff **out)
{
    size_t n = 0;
    for (size_t i = 0; i < course->scale_count; i++)
        if (course->scale[i].min > 0)
            out[n++] = &course->scale[i];
    for (size_t i = 1; i < n; i++) { /* stable insertion sort */
        const grade_cutoff *x = out[i];
        size_t j = i;
        while (j > 0 && out[j - 1]->min < x->min) {
            out[j] = out[j - 1];
            j--;
        }
        out[j] = x;
    }
    return n;
}

static bool process_course(const grade_course *course, const options *opt, bool first)
{
    grade_course_result result;
    if (grade_compute_course(course, &result) != 0) {
        fprintf(stderr, "%s: out of memory\n", progname);
        return false;
    }

    needed_answer *answers = NULL;
    size_t count = 0;
    const grade_cutoff **letters = NULL;
    size_t letter_count = 0;
    bool ok = true;
    if (opt->needed && course->scale_count > 0) {
        letters = malloc(course->scale_count * sizeof *letters);
        if (!letters)
            goto oom;
        letter_count = letter_targets(course, letters);
    }
    if (letter_count + opt->target_count > 0) {
        answers = calloc(letter_count + opt->target_count, sizeof *answers);
        if (!answers)
            goto oom;
    }
    for (size_t i = 0; i < letter_count; i++, count++) {
        answers[count].label = letters[i]->letter;
        answers[count].target = letters[i]->min;
        if (grade_needed_on_remaining(course, letters[i]->min, &answers[count].needed) != 0)
            goto oom;
    }
    for (size_t i = 0; i < opt->target_count; i++, count++) {
        answers[count].target = opt->targets[i];
        if (grade_needed_on_remaining(course, opt->targets[i], &answers[count].needed) != 0)
            goto oom;
    }

    if (opt->json) {
        course_result_write_json(stdout, &result, answers, count);
    } else {
        if (!first)
            printf("\n");
        print_text(&result, answers, count, opt);
    }
    goto done;
oom:
    fprintf(stderr, "%s: out of memory\n", progname);
    ok = false;
done:
    free(letters);
    free(answers);
    grade_course_result_free(&result);
    return ok;
}

static bool process_file(const char *path, const options *opt, bool *first)
{
    const char *shown = strcmp(path, "-") == 0 ? "<stdin>" : path;
    char err[512];
    size_t len;
    char *text = read_file(path, &len, err, sizeof err);
    if (!text) {
        fprintf(stderr, "%s: %s: %s\n", progname, shown, err);
        return false;
    }
    json_value root;
    bool parsed = json_parse(text, len, &root, err, sizeof err);
    free(text);
    if (!parsed) {
        fprintf(stderr, "%s: %s: invalid JSON: %s\n", progname, shown, err);
        return false;
    }

    bool ok = true;
    bool is_array = root.type == JSON_ARRAY;
    size_t n = is_array ? root.u.array.count : 1;
    for (size_t i = 0; i < n && ok; i++) {
        const json_value *v = is_array ? &root.u.array.items[i] : &root;
        grade_course course;
        if (!course_from_json(v, &course, err, sizeof err)) {
            if (is_array)
                fprintf(stderr, "%s: %s: course %zu: %s\n", progname, shown, i, err);
            else
                fprintf(stderr, "%s: %s: %s\n", progname, shown, err);
            ok = false;
            break;
        }
        ok = process_course(&course, opt, *first);
        *first = false;
        grade_course_free(&course);
    }
    json_free(&root);
    return ok;
}

static bool parse_target(const char *s, double *out)
{
    char *end;
    double x = strtod(s, &end);
    if (end == s || *end != '\0' || !isfinite(x))
        return false;
    *out = x;
    return true;
}

int main(int argc, char **argv)
{
    if (argc > 0 && argv[0] && argv[0][0]) {
        const char *slash = strrchr(argv[0], '/');
        progname = slash ? slash + 1 : argv[0];
    }

    options opt = { 0 };
    const char **files = calloc((size_t)argc + 1, sizeof *files);
    if (!files) {
        fprintf(stderr, "%s: out of memory\n", progname);
        return 1;
    }
    size_t file_count = 0;
    bool only_files = false;

    for (int i = 1; i < argc; i++) {
        const char *arg = argv[i];
        if (only_files || arg[0] != '-' || strcmp(arg, "-") == 0) {
            files[file_count++] = arg;
        } else if (strcmp(arg, "--") == 0) {
            only_files = true;
        } else if (strcmp(arg, "-j") == 0 || strcmp(arg, "--json") == 0) {
            opt.json = true;
        } else if (strcmp(arg, "-s") == 0 || strcmp(arg, "--summary") == 0) {
            opt.summary = true;
        } else if (strcmp(arg, "-n") == 0 || strcmp(arg, "--needed") == 0) {
            opt.needed = true;
        } else if (strcmp(arg, "-t") == 0 || strcmp(arg, "--target") == 0 ||
                   strncmp(arg, "--target=", 9) == 0) {
            const char *value = NULL;
            if (strncmp(arg, "--target=", 9) == 0)
                value = arg + 9;
            else if (i + 1 < argc)
                value = argv[++i];
            if (!value) {
                fprintf(stderr, "%s: %s needs a percentage\n", progname, arg);
                free(files);
                return 2;
            }
            if (opt.target_count == MAX_TARGETS) {
                fprintf(stderr, "%s: at most %d targets\n", progname, MAX_TARGETS);
                free(files);
                return 2;
            }
            if (!parse_target(value, &opt.targets[opt.target_count])) {
                fprintf(stderr, "%s: invalid target percentage '%s'\n", progname, value);
                free(files);
                return 2;
            }
            opt.target_count++;
        } else if (strcmp(arg, "-h") == 0 || strcmp(arg, "--help") == 0) {
            usage(stdout);
            free(files);
            return 0;
        } else if (strcmp(arg, "-V") == 0 || strcmp(arg, "--version") == 0) {
            printf("%s %s\n", progname, GRADE_VERSION);
            free(files);
            return 0;
        } else {
            fprintf(stderr, "%s: unknown option '%s'\n", progname, arg);
            usage(stderr);
            free(files);
            return 2;
        }
    }
    if (file_count == 0)
        files[file_count++] = "-";

    bool ok = true;
    bool first = true;
    for (size_t i = 0; i < file_count; i++)
        if (!process_file(files[i], &opt, &first))
            ok = false;
    free(files);
    return ok ? 0 : 1;
}
