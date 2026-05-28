use crate::sync::SyncOutcome;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToastSummary {
    pub total_lost: usize,
    pub source_count: usize,
}

/// Coalesce a batch of per-source SyncOutcomes into a single toast summary.
/// Returns None when nothing was Lost — i.e. silent successful syncs do not
/// surface a notification, matching CONTEXT.md "Loss Notification".
pub fn summarize(outcomes: &[SyncOutcome]) -> Option<ToastSummary> {
    let mut total_lost = 0usize;
    let mut source_count = 0usize;
    for o in outcomes {
        if !o.newly_lost.is_empty() {
            total_lost += o.newly_lost.len();
            source_count += 1;
        }
    }
    if total_lost == 0 {
        None
    } else {
        Some(ToastSummary {
            total_lost,
            source_count,
        })
    }
}

pub fn toast_text(s: &ToastSummary) -> (String, String) {
    let title = "Spotify Archivist".to_string();
    let body = if s.source_count == 1 {
        format!("{} track(s) lost", s.total_lost)
    } else {
        format!(
            "{} track(s) lost across {} sources",
            s.total_lost, s.source_count
        )
    };
    (title, body)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn outcome(source_id: i64, lost: &[&str]) -> SyncOutcome {
        SyncOutcome {
            source_id,
            newly_lost: lost.iter().map(|s| s.to_string()).collect(),
            newly_pending: vec![],
            cleared_pending: vec![],
            total_present: 0,
        }
    }

    #[test]
    fn no_losses_returns_none() {
        assert!(summarize(&[]).is_none());
        assert!(summarize(&[outcome(1, &[])]).is_none());
    }

    #[test]
    fn single_source_loss_summarized() {
        let s = summarize(&[outcome(1, &["t1", "t2"])]).unwrap();
        assert_eq!(s.total_lost, 2);
        assert_eq!(s.source_count, 1);
    }

    #[test]
    fn multi_source_losses_aggregated() {
        let s = summarize(&[
            outcome(1, &["t1"]),
            outcome(2, &["t2", "t3"]),
            outcome(3, &[]),
        ])
        .unwrap();
        assert_eq!(s.total_lost, 3);
        assert_eq!(s.source_count, 2);
    }

    #[test]
    fn toast_text_singular_source() {
        let (title, body) = toast_text(&ToastSummary {
            total_lost: 5,
            source_count: 1,
        });
        assert_eq!(title, "Spotify Archivist");
        assert!(body.contains("5 track"));
        assert!(!body.contains("across"));
    }

    #[test]
    fn toast_text_multi_source() {
        let (_, body) = toast_text(&ToastSummary {
            total_lost: 7,
            source_count: 3,
        });
        assert!(body.contains("across 3 sources"));
    }
}
