//! R6/R7: Pipe-owned protected capacity. Every occupancy change is one [`Event`] applied by [`apply`].
use super::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "phase", rename_all = "snake_case")]
pub enum State {
    Free,
    /// A permit exists; no launch intent, so nothing native can have started.
    Held {
        op: String,
    },
    /// One-shot launch intent recorded; native work may exist until proved otherwise.
    Launched {
        op: String,
    },
    /// Native, descendant and cleanup proof verified; only release remains.
    Completed {
        op: String,
    },
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    Acquire {
        op: String,
    },
    LaunchIntended {
        op: String,
    },
    /// Cancel acknowledgment, transport exit, crash or unknown reconciliation: proves nothing.
    Unknown {
        op: String,
    },
    CompletionVerified {
        op: String,
    },
    Release {
        op: String,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reject {
    Busy,
    Foreign,
    Replay,
    Unproved,
}

impl std::fmt::Display for Reject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Reject::Busy => "busy",
            Reject::Foreign => "foreign",
            Reject::Replay => "replay",
            Reject::Unproved => "unproved",
        })
    }
}

pub fn apply(state: &State, event: &Event) -> Result<State, Reject> {
    use {Event as E, State as S};
    let holder = match state {
        S::Free => None,
        S::Held { op } | S::Launched { op } | S::Completed { op } => Some(op),
    };
    let op = match event {
        E::Acquire { op } | E::LaunchIntended { op } | E::Unknown { op } | E::CompletionVerified { op } | E::Release { op } => op,
    };
    if holder.is_some_and(|holder| holder != op) {
        return Err(if matches!(event, E::Acquire { .. }) { Reject::Busy } else { Reject::Foreign });
    }
    match (state, event) {
        (S::Free, E::Acquire { op }) => Ok(S::Held { op: op.clone() }),
        (S::Free, _) => Err(Reject::Foreign),
        (S::Held { .. }, E::LaunchIntended { op }) => Ok(S::Launched { op: op.clone() }),
        (S::Launched { .. } | S::Completed { .. }, E::LaunchIntended { .. }) => Err(Reject::Replay),
        (S::Held { .. }, E::CompletionVerified { .. }) => Err(Reject::Unproved),
        (S::Launched { .. } | S::Completed { .. }, E::CompletionVerified { op }) => Ok(S::Completed { op: op.clone() }),
        (S::Held { .. } | S::Completed { .. }, E::Release { .. }) => Ok(S::Free),
        (S::Launched { .. }, E::Release { .. }) => Err(Reject::Unproved),
        (_, E::Acquire { .. } | E::Unknown { .. }) => Ok(state.clone()),
    }
}

/// Replay one capacity trace: `reject` steps must be refused with that reason and change nothing.
pub fn replay(raw: &str) -> Res<()> {
    let trace: Value = serde_json::from_str(raw).map_err(|e| e.to_string())?;
    let mut state = State::Free;
    for (i, step) in trace["steps"].as_array().ok_or("steps missing")?.iter().enumerate() {
        let event: Event = serde_json::from_value(step["event"].clone()).map_err(|e| format!("step {i}: {e}"))?;
        match (step["reject"].as_str(), apply(&state, &event)) {
            (None, Ok(next)) => state = next,
            (Some(want), Err(got)) if got.to_string() == want => {}
            (want, got) => return Err(format!("step {i}: expected {want:?}, got {got:?}")),
        }
    }
    if serde_json::to_value(&state).map_err(|e| e.to_string())? != trace["expect"] {
        return Err(format!("expected {}, got {state:?}", trace["expect"]));
    }
    Ok(())
}

/// The durable state of one installation's permit: an absent file is free.
pub(super) struct Capacity(pub PathBuf);

impl Capacity {
    fn load(&self) -> Res<State> {
        if !self.0.try_exists().map_err(|_| "protected capacity unknown; no effect")? {
            return Ok(State::Free);
        }
        serde_json::from_value(read_json(&self.0)?).map_err(|_| "protected capacity corrupt; retain occupancy".into())
    }

    pub(super) fn holds(&self, op: &str) -> Res<bool> {
        Ok(match self.load()? {
            State::Free => false,
            State::Held { op: holder } | State::Launched { op: holder } | State::Completed { op: holder } => holder == op,
        })
    }

    /// Only the holder writes after acquisition, and the link makes first acquisition atomic.
    pub(super) fn apply(&self, event: Event) -> Res<()> {
        let refuse = |reject: Reject| match reject {
            Reject::Busy => "capability_unavailable: protected permit held by another operation; no launch".to_string(),
            other => format!("protected permit transition refused: {other}; retain occupancy"),
        };
        let before = self.load()?;
        match apply(&before, &event).map_err(refuse)? {
            next if next == before => Ok(()),
            State::Free => {
                fs::remove_file(&self.0).map_err(|e| e.to_string())?;
                sync_spool(self.0.parent().ok_or("protected capacity parent unknown")?)
            }
            next => {
                let value = serde_json::to_value(&next).map_err(|e| e.to_string())?;
                if before != State::Free {
                    return save(&self.0, &value);
                }
                let draft = self.0.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
                save(&draft, &value)?;
                let linked = fs::hard_link(&draft, &self.0);
                let _ = fs::remove_file(&draft);
                match linked {
                    Ok(()) => sync_spool(self.0.parent().ok_or("protected capacity parent unknown")?),
                    Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => match apply(&self.load()?, &event) {
                        Ok(_) => Ok(()),
                        Err(reject) => Err(refuse(reject)),
                    },
                    Err(e) => Err(e.to_string()),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // R6/R7: the durable file follows the model; a second operation cannot take or free occupancy.
    #[test]
    fn image_worker_capacity_file_follows_model() {
        let dir = std::env::temp_dir().join(format!("pipe-capacity-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&dir).unwrap();
        let capacity = Capacity(dir.join("capacity.json"));
        let (a, b) = (|| "invented-a".to_string(), || "invented-b".to_string());
        capacity.apply(Event::Acquire { op: a() }).unwrap();
        capacity.apply(Event::Acquire { op: a() }).unwrap();
        assert!(capacity.apply(Event::Acquire { op: b() }).unwrap_err().contains("held by another operation"));
        assert!(capacity.holds(&a()).unwrap() && !capacity.holds(&b()).unwrap());
        capacity.apply(Event::LaunchIntended { op: a() }).unwrap();
        for refused in [Event::Release { op: a() }, Event::Release { op: b() }, Event::LaunchIntended { op: a() }] {
            assert!(capacity.apply(refused).is_err());
        }
        capacity.apply(Event::Unknown { op: a() }).unwrap();
        assert!(capacity.0.exists());
        capacity.apply(Event::CompletionVerified { op: a() }).unwrap();
        capacity.apply(Event::Release { op: a() }).unwrap();
        assert!(!capacity.0.exists());
        capacity.apply(Event::Acquire { op: b() }).unwrap();
        fs::write(&capacity.0, b"{").unwrap();
        assert!(capacity.apply(Event::Acquire { op: a() }).is_err(), "corrupt occupancy is never free");
        fs::remove_dir_all(dir).unwrap();
    }

    // R6/R7: two racing operations produce one holder.
    #[test]
    fn image_worker_capacity_acquire_is_atomic() {
        let dir = std::env::temp_dir().join(format!("pipe-capacity-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&dir).unwrap();
        let path = dir.join("capacity.json");
        let racers: Vec<_> = (0..16)
            .map(|i| {
                let path = path.clone();
                std::thread::spawn(move || Capacity(path).apply(Event::Acquire { op: format!("invented-{i}") }).is_ok())
            })
            .collect();
        assert_eq!(racers.into_iter().map(|r| r.join().unwrap()).filter(|won| *won).count(), 1);
        fs::remove_dir_all(dir).unwrap();
    }
}
