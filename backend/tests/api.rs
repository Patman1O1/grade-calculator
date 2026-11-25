//! Tests of the operations the front-end calls (through `wasm-bindgen`), and of
//! the JSON shapes they produce.

mod fixtures;

use grade_calculator::api::{NOT_A_COURSE, evaluate, import_courses, summaries};
use grade_calculator::course::Course;
use grade_calculator::status::Status;
use serde_json::{Value, json};

fn to_json<T: serde::Serialize>(v: &T) -> Value {
    serde_json::to_value(v).unwrap()
}

#[test]
fn blank_course_is_ungraded() {
    let course = to_json(&Course::blank());
    assert_eq!(course["name"], "New course");
    assert_eq!(course["scheme"], "weighted");
    assert_eq!(course["categories"].as_array().unwrap().len(), 2);
    assert_eq!(
        course["scale"],
        json!({ "a": 90.0, "b": 80.0, "c": 70.0, "d": 60.0, "f": 0.0 })
    );
    // Ids belong to the front-end; the engine has none.
    assert!(course.get("id").is_none());
    assert!(course["categories"][0].get("id").is_none());
    assert!(course["categories"][0]["items"][0].get("id").is_none());
    // Ungraded courses carry no computed fields; an item's status is stored.
    assert!(course.get("percent").is_none());
    assert!(course.get("target").is_none() && course.get("needs").is_none());
    assert!(course["categories"][0].get("earned").is_none());
    assert_eq!(course["categories"][0]["items"][0]["status"], "ungraded");
}

#[test]
fn evaluates_a_course_and_solves_targets() {
    let course: Course = serde_json::from_value(json!({
        "id": "c", "name": "Test", "scheme": "points",
        "scale": { "a": 90, "b": 80, "c": 70, "d": 65, "f": 0 },
        "target": 30,
        "categories": [{
            "id": "k", "name": "All", "weight": 0,
            "items": [
                { "id": "a", "name": "A", "earnedPoints": 60, "totalPoints": 100 },
                { "id": "b", "name": "B", "earnedPoints": null, "totalPoints": 100 }
            ]
        }]
    }))
    .unwrap();
    let graded = to_json(&evaluate(&course));

    assert_eq!(graded["name"], "Test");
    assert_eq!(graded["percent"], 60.0);
    assert_eq!(graded["letter"], "F");
    assert_eq!(graded["earned"], 60.0);
    assert_eq!(graded["categories"][0]["percent"], 60.0);
    let items = &graded["categories"][0]["items"];
    assert_eq!(items[0]["status"], "counted");
    assert_eq!(items[0]["earnedPoints"], 60.0);
    assert_eq!(items[0]["totalPoints"], 100.0);
    assert_eq!(items[1]["status"], "ungraded");
    assert_eq!(items[1]["earnedPoints"], Value::Null);

    // Targets: the letters above 0, highest first, then the course's own 30%.
    assert_eq!(graded["target"], 30.0);
    assert_eq!(graded["remaining"], 1);
    assert_eq!(graded["best"], 80.0);
    let needs: Vec<Option<f64>> = serde_json::from_value(graded["needs"].clone()).unwrap();
    assert_eq!(needs.len(), 5);
    assert_eq!(needs[0], None); // A (90%): out of reach
    for (need, expected) in needs[1..4].iter().zip([1.0, 0.8, 0.7]) {
        assert!(
            (need.unwrap() - expected).abs() < 1e-6,
            "{need:?} vs {expected}"
        );
    }
    assert_eq!(needs[4], Some(0.0)); // 30%: secured
}

#[test]
fn summarises_every_course() {
    let c1 = fixtures::cs401();
    let c2 = Course::blank();
    // Summaries come back in the same order as the courses.
    let out = summaries(vec![c1, c2]);
    assert_eq!(out.len(), 2);
    assert!((out[0].unwrap() - 56.95).abs() < 0.005);
    assert_eq!(out[1], None);
    // The front-end relies on `null` here.
    assert_eq!(to_json(&out), json!([out[0], null]));
}

#[test]
fn import_validates_courses_and_drops_ids() {
    let mut course = to_json(&fixtures::math218());
    // Exports carry the front-end's ids; the engine ignores them.
    course["id"] = json!("course-1");
    course["categories"][0]["id"] = json!("cat-1");
    course["categories"][0]["items"][0]["id"] = json!("item-1");
    let imported = import_courses(json!([course, { "not": "a course" }])).unwrap();
    assert_eq!(imported.len(), 1);
    assert_eq!(imported[0].name, course["name"]);
    let out = to_json(&imported[0]);
    assert!(out.get("id").is_none());
    assert!(out["categories"][0].get("id").is_none());
    assert!(out["categories"][0]["items"][0].get("id").is_none());

    assert_eq!(import_courses(json!({ "hello": 1 })), Err(NOT_A_COURSE));
    assert_eq!(import_courses(json!(42)), Err(NOT_A_COURSE));
}

#[test]
fn computed_fields_in_input_are_recomputed() {
    let forged = json!({ "name": "F", "scheme": "points", "scale": { "a": 90, "b": 80, "c": 70, "d": 60, "f": 0 }, "percent": 100,
        "remaining": 7, "best": 100, "needs": [0.5], "letter": "A",
        "categories": [{ "name": "C", "weight": 0, "earned": 99,
            "items": [
                { "name": "I", "earnedPoints": 0, "totalPoints": 10, "status": "dropped" },
                { "name": "J", "earnedPoints": 5, "totalPoints": 10, "status": "excluded" }
            ] }] });

    let imported = import_courses(forged).unwrap();
    assert_eq!(imported[0].percent, None);
    assert_eq!(
        (
            &imported[0].remaining,
            &imported[0].best,
            &imported[0].needs
        ),
        (&None, &None, &None)
    );
    assert_eq!(imported[0].categories[0].earned, None);
    assert_eq!(imported[0].categories[0].items[0].status, Status::Counted);
    assert_eq!(imported[0].categories[0].items[1].status, Status::Excluded);

    let graded = evaluate(&imported[0]);
    assert_eq!(graded.remaining, Some(0));
    assert_eq!(graded.percent, Some(0.0));
    assert_eq!(graded.letter.as_deref(), Some("F"));
    assert_eq!(graded.categories[0].items[0].earned_points, Some(0.0));
    assert_eq!(graded.categories[0].items[0].status, Status::Counted);
    // The excluded item is kept out: 0/10, not 5/20.
    assert_eq!(graded.categories[0].items[1].status, Status::Excluded);
    assert_eq!((graded.earned, graded.possible), (Some(0.0), Some(10.0)));
}
