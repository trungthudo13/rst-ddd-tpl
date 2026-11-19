use uuid::Uuid;

use crate::{
    Error,
    Item,
    ItemSnapshot,
    mappers::item::{
        from_snapshot,
        to_snapshot,
    },
};

#[test]
fn snapshots_preserve_values_and_use_validated_reconstruction() {
    let item = Item::new(Uuid::nil(), "résumé 🦀".into(), i64::MAX).unwrap();
    assert_eq!(from_snapshot(to_snapshot(&item)).unwrap(), item);
    let invalid = ItemSnapshot {
        id: item.id(),
        title: " \t".into(),
        created_at: item.created_at(),
    };
    assert_eq!(from_snapshot(invalid), Err(Error::EmptyTitle));
}
