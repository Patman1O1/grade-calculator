use super::*;

// ── Helpers ─────────────────────────────────────────────────────────────────
fn item(earned_points: Option<f64>, total_points: f64) -> Item {
    Item::new(
        "Quiz",
        earned_points,
        total_points,
        false,
        None,
        Status::default(),
    )
}

// ── `Item` Tests ────────────────────────────────────────────────────────────
mod item {
    use super::{Item, Status, item};

    // ── Function Tests ──────────────────────────────────────────────────────
    mod functions {
        use super::{Item, Status};

        // ── `new()` Tests ───────────────────────────────────────────────────
        #[test]
        fn new() {
            let i = Item::new(
                "Exam",
                Some(8.5),
                10.0,
                true,
                Some("note".into()),
                Status::Counted,
            );
            assert_eq!(i.name, "Exam");
            assert_eq!(i.earned_points, Some(8.5));
            assert_eq!(i.total_points, 10.0);
            assert!(i.never_drop);
            assert_eq!(i.note.as_deref(), Some("note"));
            assert_eq!(i.status, Status::Counted);
        }

        #[test]
        fn new_accepts_owned_and_borrowed_names() {
            assert_eq!(
                Item::new(String::from("A"), None, 1.0, false, None, Status::Ungraded).name,
                "A"
            );
            assert_eq!(
                Item::new("B", None, 1.0, false, None, Status::Ungraded).name,
                "B"
            );
        }
    }

    // ── Method Tests ────────────────────────────────────────────────────────
    mod methods {
        use super::{Status, item};

        // ── `drop()` Tests ──────────────────────────────────────────────────
        mod drop {
            use super::{Status, item};

            #[test]
            fn drops_a_droppable_item() {
                let mut i = item(Some(3.0), 10.0);
                i.status = Status::Counted;
                assert!(i.drop());
                assert_eq!(i.status, Status::Dropped);
                // The score is kept so the UI can still show it.
                assert_eq!((i.earned_points, i.total_points), (Some(3.0), 10.0));
            }

            #[test]
            fn refuses_a_never_drop_item() {
                let mut i = item(Some(3.0), 10.0);
                i.never_drop = true;
                i.status = Status::Counted;
                assert!(!i.drop());
                assert_eq!(i.status, Status::Counted);
            }
        }

        // ── `exclude()` Tests ───────────────────────────────────────────────
        #[test]
        fn exclude() {
            let mut i = item(Some(3.0), 10.0);
            i.exclude();
            assert_eq!(i.status, Status::Excluded);
            assert_eq!((i.earned_points, i.total_points), (Some(3.0), 10.0));
        }

        // ── `ungrade()` Tests ───────────────────────────────────────────────
        #[test]
        fn ungrade() {
            let mut i = item(Some(3.0), 10.0);
            i.status = Status::Counted;
            assert_eq!(i.ungrade(), (Some(3.0), 10.0));
            // Only the score is cleared; the item is still out of 10.
            assert_eq!((i.earned_points, i.total_points), (None, 10.0));
            assert_eq!(i.status, Status::Ungraded);
            assert_eq!(i.percent(), None);
        }

        // ── `grade()` Tests ─────────────────────────────────────────────────
        #[test]
        fn grade() {
            let mut i = item(None, 10.0);
            i.grade(8.0, 20.0);
            assert_eq!((i.earned_points, i.total_points), (Some(8.0), 20.0));
            assert_eq!(i.status, Status::Counted);
        }

        // ── `status()` Tests ────────────────────────────────────────────────
        #[test]
        fn status() {
            let mut i = item(None, 10.0);
            assert_eq!(i.status(), Status::Ungraded);
            i.exclude();
            assert_eq!(i.status(), Status::Excluded);
        }

        // ── `percent()` Tests ───────────────────────────────────────────────
        mod percent {
            use super::item;

            #[test]
            fn graded() {
                assert_eq!(item(Some(8.5), 10.0).percent(), Some(85.0));
                assert_eq!(item(Some(0.0), 10.0).percent(), Some(0.0));
            }

            #[test]
            fn ungraded_is_none() {
                assert_eq!(item(None, 10.0).percent(), None);
            }

            #[test]
            fn extra_credit_is_none() {
                assert_eq!(item(Some(2.0), 0.0).percent(), None);
            }
        }
    }

    // ── Trait Tests ─────────────────────────────────────────────────────────
    mod traits {
        use super::{Item, Status, item};

        use serde_json::{Value, json};

        // ── `Default` Tests ─────────────────────────────────────────────────
        #[test]
        fn default() {
            let i = Item::default();
            assert_eq!(i.name, "");
            assert_eq!((i.earned_points, i.total_points), (None, 0.0));
            assert!(!i.never_drop);
            assert_eq!(i.note, None);
            assert_eq!(i.status, Status::Ungraded);
        }

        // ── `Serialize` Tests ───────────────────────────────────────────────
        #[test]
        fn serialize() {
            let mut i = item(Some(1.0), 2.0);
            i.status = Status::Counted;
            assert_eq!(
                serde_json::to_value(&i).unwrap(),
                json!({ "name": "Quiz", "earnedPoints": 1.0, "totalPoints": 2.0, "status": "counted" })
            );
            // Ungraded is an explicit null, so it survives a round trip;
            // neverDrop and note are only written when set.
            let mut i = item(None, 10.0);
            i.never_drop = true;
            i.note = Some("n".into());
            let v = serde_json::to_value(&i).unwrap();
            assert_eq!(v["earnedPoints"], Value::Null);
            assert_eq!(v["neverDrop"], true);
            assert_eq!(v["note"], "n");
        }

        // ── `Deserialize` Tests ─────────────────────────────────────────────
        mod deserialize {
            use super::{Item, Status};

            use serde_json::{from_value, json};

            #[test]
            fn round_trip() {
                let i = Item::new(
                    "Q",
                    Some(3.0),
                    10.0,
                    true,
                    Some("n".into()),
                    Status::Excluded,
                );
                let back: Item = from_value(serde_json::to_value(&i).unwrap()).unwrap();
                assert_eq!(back, i);
            }

            #[test]
            fn optional_fields_default() {
                let i: Item = from_value(json!({ "name": "Q", "totalPoints": 10 })).unwrap();
                assert_eq!(i.earned_points, None);
                assert!(!i.never_drop);
                assert_eq!(i.note, None);
                assert_eq!(i.status, Status::Ungraded);
            }

            #[test]
            fn requires_total_points() {
                assert!(from_value::<Item>(json!({ "name": "?", "earnedPoints": 5 })).is_err());
            }
        }
    }
}
