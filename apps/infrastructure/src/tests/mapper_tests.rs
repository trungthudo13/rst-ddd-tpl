use domain::Item;
use shared::Error;
use uuid::Uuid;

use crate::{
    adapters::hiqlite::item_repository::mappers::item::{
        from_row,
        to_row,
    },
    models::item::ItemRow,
};

#[test]
fn persisted_rows_preserve_values_and_reject_corrupt_data() {
    let item = Item::new(Uuid::nil(), "résumé 🦀".into(), i64::MAX).unwrap();
    assert_eq!(from_row(to_row(&item)).unwrap(), item);
    let invalid = ItemRow {
        id: "invalid".into(),
        title: "valid".into(),
        created_at: 1,
    };
    assert!(matches!(from_row(invalid), Err(Error::Unexpected(_))));
    let invalid = ItemRow {
        id: item.id().to_string(),
        title: " \t".into(),
        created_at: 1,
    };
    assert!(matches!(from_row(invalid), Err(Error::Unexpected(_))));
}
