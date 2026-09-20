//! The field journal: what the astronaut has discovered.
//!
//! Milestone 5, step 3 fills in the pages. For now this holds the event that
//! fires the first time an object type is touched.

use bevy::prelude::*;

use crate::objects::ObjectKind;

/// Sent the first time an object type is ever touched (across all sessions).
#[derive(Message, Debug, Clone, Copy)]
pub(crate) struct ObjectDiscovered(
    #[expect(dead_code, reason = "the journal pages read it in step 5c")] pub ObjectKind,
);

/// The field journal.
pub(crate) struct JournalPlugin;

impl Plugin for JournalPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ObjectDiscovered>();
    }
}
