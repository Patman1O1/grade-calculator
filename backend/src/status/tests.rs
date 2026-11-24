use super::*;

// ── `Status` Tests ──────────────────────────────────────────────────────────
mod status {
    use super::Status;

    // ── Method Tests ────────────────────────────────────────────────────────
    mod methods {
        use super::Status;

        // ── `is_counted()` Tests ────────────────────────────────────────────
        mod is_counted {
            use super::Status;

            #[test]
            fn true_case() {
                assert!(Status::Counted.is_counted());
            }

            #[test]
            fn false_cases() {
                assert!(!Status::Dropped.is_counted());
                assert!(!Status::Excluded.is_counted());
                assert!(!Status::Ungraded.is_counted());
            }
        }

        // ── `is_dropped()` Tests ────────────────────────────────────────────
        mod is_dropped {
            use super::Status;

            #[test]
            fn true_case() {
                assert!(Status::Dropped.is_dropped());
            }

            #[test]
            fn false_cases() {
                assert!(!Status::Counted.is_dropped());
                assert!(!Status::Excluded.is_dropped());
                assert!(!Status::Ungraded.is_dropped());
            }
        }

        // ── `is_excluded()` Tests ───────────────────────────────────────────
        mod is_excluded {
            use super::Status;

            #[test]
            fn true_case() {
                assert!(Status::Excluded.is_excluded());
            }

            #[test]
            fn false_cases() {
                assert!(!Status::Counted.is_excluded());
                assert!(!Status::Dropped.is_excluded());
                assert!(!Status::Ungraded.is_excluded());
            }
        }

        // ── `is_ungraded()` Tests ───────────────────────────────────────────
        mod is_ungraded {
            use super::Status;

            #[test]
            fn true_case() {
                assert!(Status::Ungraded.is_ungraded());
            }

            #[test]
            fn false_cases() {
                assert!(!Status::Counted.is_ungraded());
                assert!(!Status::Dropped.is_ungraded());
                assert!(!Status::Excluded.is_ungraded());
            }
        }
    }

    // ── Trait Tests ─────────────────────────────────────────────────────────
    mod traits {
        use super::Status;

        // ── `Default` Tests ─────────────────────────────────────────────────
        #[test]
        fn default() {
            assert_eq!(Status::Ungraded, Status::default());
        }

        // ── `Clone` Tests ───────────────────────────────────────────────────
        #[test]
        #[allow(clippy::clone_on_copy)]
        fn clone() {
            assert_eq!(Status::Counted.clone(), Status::Counted);
            assert_eq!(Status::Dropped.clone(), Status::Dropped);
            assert_eq!(Status::Excluded.clone(), Status::Excluded);
            assert_eq!(Status::Ungraded.clone(), Status::Ungraded);
        }

        // ── `Copy` Tests ────────────────────────────────────────────────────
        #[test]
        fn copy() {
            let original = Status::Excluded;
            let copied = original;
            assert_eq!(original, copied);
        }

        // ── `PartialEq` / `Eq` Tests ────────────────────────────────────────
        mod eq {
            use super::Status;

            #[test]
            fn true_case() {
                assert_eq!(Status::Counted, Status::Counted);
                assert_eq!(Status::Dropped, Status::Dropped);
                assert_eq!(Status::Excluded, Status::Excluded);
                assert_eq!(Status::Ungraded, Status::Ungraded);
            }

            #[test]
            fn false_cases() {
                assert_ne!(Status::Counted, Status::Dropped);
                assert_ne!(Status::Counted, Status::Excluded);
                assert_ne!(Status::Counted, Status::Ungraded);
                assert_ne!(Status::Dropped, Status::Excluded);
                assert_ne!(Status::Dropped, Status::Ungraded);
                assert_ne!(Status::Excluded, Status::Ungraded);
            }
        }

        // ── `Display` Tests ─────────────────────────────────────────────────
        mod display {
            use super::Status;

            #[test]
            fn counted() {
                assert_eq!(Status::Counted.to_string(), "Counted");
            }

            #[test]
            fn dropped() {
                assert_eq!(Status::Dropped.to_string(), "Dropped");
            }

            #[test]
            fn excluded() {
                assert_eq!(Status::Excluded.to_string(), "Excluded");
            }

            #[test]
            fn ungraded() {
                assert_eq!(Status::Ungraded.to_string(), "Ungraded");
            }
        }

        // ── `Debug` Tests ───────────────────────────────────────────────────
        mod debug {
            use super::Status;

            #[test]
            fn counted() {
                assert_eq!(
                    format!("{:?}", Status::Counted),
                    format!("{}", Status::Counted)
                );
            }

            #[test]
            fn dropped() {
                assert_eq!(
                    format!("{:?}", Status::Dropped),
                    format!("{}", Status::Dropped)
                );
            }

            #[test]
            fn excluded() {
                assert_eq!(
                    format!("{:?}", Status::Excluded),
                    format!("{}", Status::Excluded)
                );
            }

            #[test]
            fn ungraded() {
                assert_eq!(
                    format!("{:?}", Status::Ungraded),
                    format!("{}", Status::Ungraded)
                );
            }

            #[test]
            fn pretty_matches_display() {
                assert_eq!(
                    format!("{:#?}", Status::Counted),
                    format!("{}", Status::Counted)
                );
            }
        }

        // ── `Hash` Tests ────────────────────────────────────────────────────
        mod hash {
            // ── Aliases ─────────────────────────────────────────────────────
            use super::Status;

            use core::hash::{Hash, Hasher};

            use std::collections::{HashSet, hash_map::DefaultHasher};

            // ── Functions ───────────────────────────────────────────────────
            fn hash_of(status: Status) -> u64 {
                let mut hasher = DefaultHasher::new();
                status.hash(&mut hasher);
                hasher.finish()
            }

            // ── Tests ───────────────────────────────────────────────────────
            #[test]
            fn equal_values_hash_equal() {
                assert_eq!(hash_of(Status::Counted), hash_of(Status::Counted));
                assert_eq!(hash_of(Status::Dropped), hash_of(Status::Dropped));
                assert_eq!(hash_of(Status::Excluded), hash_of(Status::Excluded));
                assert_eq!(hash_of(Status::Ungraded), hash_of(Status::Ungraded));
            }

            #[test]
            fn distinct_values_hash_distinct() {
                let hashes: HashSet<u64> = [
                    Status::Counted,
                    Status::Dropped,
                    Status::Excluded,
                    Status::Ungraded,
                ]
                .into_iter()
                .map(hash_of)
                .collect();

                assert_eq!(hashes.len(), 4);
            }

            #[test]
            fn hash_set_key() {
                let mut set = HashSet::new();
                assert!(set.insert(Status::Counted));
                assert!(set.insert(Status::Dropped));
                assert!(!set.insert(Status::Counted));

                assert!(set.contains(&Status::Counted));
                assert!(set.contains(&Status::Dropped));
                assert!(!set.contains(&Status::Excluded));
                assert_eq!(set.len(), 2);
            }
        }

        // ── `Serialize` Tests ───────────────────────────────────────────────
        #[test]
        fn serialize() {
            assert_eq!(
                serde_json::to_string(&Status::Counted).unwrap(),
                "\"counted\""
            );
            assert_eq!(
                serde_json::to_string(&Status::Dropped).unwrap(),
                "\"dropped\""
            );
            assert_eq!(
                serde_json::to_string(&Status::Excluded).unwrap(),
                "\"excluded\""
            );
            assert_eq!(
                serde_json::to_string(&Status::Ungraded).unwrap(),
                "\"ungraded\""
            );
        }

        // ── `Deserialize` Tests ─────────────────────────────────────────────
        mod deserialize {
            // ── Aliases ─────────────────────────────────────────────────────
            use super::Status;

            use serde_json::from_str;

            // ── Tests ───────────────────────────────────────────────────────
            #[test]
            fn valid_values() {
                assert_eq!(from_str::<Status>("\"counted\"").unwrap(), Status::Counted);
                assert_eq!(from_str::<Status>("\"dropped\"").unwrap(), Status::Dropped);
                assert_eq!(
                    from_str::<Status>("\"excluded\"").unwrap(),
                    Status::Excluded
                );
                assert_eq!(
                    from_str::<Status>("\"ungraded\"").unwrap(),
                    Status::Ungraded
                );
            }

            #[test]
            fn invalid_values() {
                assert!(from_str::<Status>("\"Counted\"").is_err());
                assert!(from_str::<Status>("\"COUNTED\"").is_err());
                assert!(from_str::<Status>("\"graded\"").is_err());
                assert!(from_str::<Status>("\"\"").is_err());
                assert!(from_str::<Status>("0").is_err());
                assert!(from_str::<Status>("null").is_err());
            }
        }
    }
}
