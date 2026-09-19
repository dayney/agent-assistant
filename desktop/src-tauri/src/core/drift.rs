#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Class {
    Clean,
    Pending,
    Drift,
    Converged,
    Conflict,
    New,
    ForeignCollision,
    Orphan,
    OrphanDrifted,
}

pub(crate) fn classify(source: &str, applied: &str, destination: &str) -> Class {
    if applied.is_empty() && destination.is_empty() && !source.is_empty() {
        Class::New
    } else if applied.is_empty() && !destination.is_empty() && !source.is_empty() {
        Class::ForeignCollision
    } else if !applied.is_empty() && source.is_empty() {
        if destination == applied {
            Class::Orphan
        } else {
            Class::OrphanDrifted
        }
    } else if source == applied && destination == applied {
        Class::Clean
    } else if source != applied && destination == applied {
        Class::Pending
    } else if source == applied && destination != applied {
        Class::Drift
    } else if source != applied && destination != applied && source == destination {
        Class::Converged
    } else {
        Class::Conflict
    }
}

pub(crate) fn safe_for_auto_apply(class: Class) -> bool {
    matches!(
        class,
        Class::Clean | Class::Pending | Class::New | Class::Converged
    )
}

pub(crate) fn line_diff(actual: &str, desired: &str) -> String {
    if actual == desired {
        return String::new();
    }
    let diff = similar::TextDiff::from_lines(actual, desired);
    let mut output = String::new();
    for change in diff.iter_all_changes() {
        let prefix = match change.tag() {
            similar::ChangeTag::Delete => '-',
            similar::ChangeTag::Insert => '+',
            similar::ChangeTag::Equal => ' ',
        };
        output.push(prefix);
        output.push_str(change.value());
        if !change.value().ends_with('\n') {
            output.push('\n');
        }
    }
    output
}

impl Class {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Clean => "clean",
            Self::Pending => "pending",
            Self::Drift => "drift",
            Self::Converged => "converged",
            Self::Conflict => "conflict",
            Self::New => "new",
            Self::ForeignCollision => "foreign-collision",
            Self::Orphan => "orphan",
            Self::OrphanDrifted => "orphan-drifted",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{classify, line_diff, safe_for_auto_apply, Class};

    #[test]
    fn classifies_all_go_baseline_cases() {
        let cases = [
            ("a", "a", "a", Class::Clean),
            ("b", "a", "a", Class::Pending),
            ("a", "a", "b", Class::Drift),
            ("b", "a", "b", Class::Converged),
            ("b", "a", "c", Class::Conflict),
            ("a", "", "", Class::New),
            ("a", "", "x", Class::ForeignCollision),
            ("", "a", "a", Class::Orphan),
            ("", "a", "b", Class::OrphanDrifted),
            ("a", "a", "", Class::Drift),
            ("b", "a", "", Class::Conflict),
        ];
        for (source, applied, destination, expected) in cases {
            assert_eq!(classify(source, applied, destination), expected);
        }
    }

    #[test]
    fn only_non_destructive_classes_auto_apply() {
        for class in [Class::Clean, Class::Pending, Class::New, Class::Converged] {
            assert!(safe_for_auto_apply(class));
        }
        for class in [
            Class::Drift,
            Class::Conflict,
            Class::ForeignCollision,
            Class::Orphan,
            Class::OrphanDrifted,
        ] {
            assert!(!safe_for_auto_apply(class));
        }
    }

    #[test]
    fn diff_reports_removed_and_added_lines() {
        let diff = line_diff("old\n", "new\n");
        assert!(diff.contains("-old"));
        assert!(diff.contains("+new"));
    }
}
