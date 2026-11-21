//! Unit tests for [`super`] (`category/mod.rs`).

use super::*;
use crate::item::Status;
use crate::round;

fn item(score: Option<f64>, out_of: f64) -> Item {
    Item {
        score,
        out_of,
        ..Default::default()
    }
}

fn graded(items: Vec<Item>, drop_lowest: f64, drop_highest: f64) -> Category {
    let mut c = Category {
        items,
        drop_lowest: Some(drop_lowest),
        drop_highest: Some(drop_highest),
        ..Default::default()
    };
    c.grade();
    c
}

fn dropped(c: &Category) -> Vec<bool> {
    c.items
        .iter()
        .map(|i| i.status == Some(Status::Dropped))
        .collect()
}

#[test]
fn drops_the_item_that_maximises_the_result_not_just_the_lowest_percentage() {
    // Dropping the 0/1 gives 40/110 = 36%; dropping the 30/100 gives 10/11 = 91%.
    let c = graded(
        vec![
            item(Some(0.0), 1.0),
            item(Some(30.0), 100.0),
            item(Some(10.0), 10.0),
        ],
        1.0,
        0.0,
    );
    assert_eq!(dropped(&c), [false, true, false]);
    assert_eq!(round(c.percent.unwrap(), 2), 90.91);
}

#[test]
fn drop_highest_removes_the_best_result() {
    let c = graded(
        vec![
            item(Some(10.0), 10.0),
            item(Some(5.0), 10.0),
            item(Some(7.0), 10.0),
        ],
        0.0,
        1.0,
    );
    assert_eq!(dropped(&c), [true, false, false]);
    assert_eq!(c.percent, Some(60.0));
}

#[test]
fn never_drops_never_drop_items_and_always_keeps_one() {
    let mut keep = item(Some(0.0), 10.0);
    keep.never_drop = true;
    let c = graded(vec![keep, item(Some(5.0), 10.0)], 1.0, 0.0);
    assert_eq!(dropped(&c), [false, true]);

    let single = graded(vec![item(Some(2.0), 10.0)], 3.0, 0.0);
    assert_eq!(single.percent, Some(20.0));
}

#[test]
fn ignores_ungraded_and_excluded_items_when_dropping() {
    let mut excluded = item(Some(1.0), 10.0);
    excluded.excluded = true;
    let c = graded(
        vec![
            item(None, 10.0),
            excluded,
            item(Some(4.0), 10.0),
            item(Some(8.0), 10.0),
        ],
        1.0,
        0.0,
    );
    assert_eq!(c.percent, Some(80.0));
    let statuses: Vec<_> = c.items.iter().map(|i| i.status.unwrap()).collect();
    assert_eq!(
        statuses,
        [
            Status::Ungraded,
            Status::Excluded,
            Status::Dropped,
            Status::Counted
        ]
    );
}

#[test]
fn sums_counted_items_including_extra_credit() {
    let c = graded(vec![item(Some(9.0), 10.0), item(Some(2.0), 0.0)], 0.0, 0.0);
    assert_eq!((c.earned, c.possible), (Some(11.0), Some(10.0)));
    let empty = graded(vec![item(None, 10.0)], 0.0, 0.0);
    assert_eq!((empty.earned, empty.percent), (Some(0.0), None));
}
