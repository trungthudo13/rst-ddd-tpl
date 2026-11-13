use uuid::Uuid;

use crate::{
    DomainError,
    Item,
};

#[test]
fn trims_title_and_rejects_blank_titles() {
    let item = Item::new(Uuid::new_v4(), "  first item  ".into(), 1).unwrap();
    assert_eq!(item.title(), "first item");
    assert_eq!(
        Item::new(Uuid::new_v4(), "   ".into(), 1),
        Err(DomainError::EmptyTitle)
    );
}

#[test]
fn rename_preserves_identity_and_creation_time() {
    let id = Uuid::new_v4();
    let mut item = Item::new(id, "first".into(), 123).unwrap();

    item.rename("  renamed  ".into()).unwrap();

    assert_eq!(item.title(), "renamed");
    assert_eq!(item.id(), id);
    assert_eq!(item.created_at(), 123);
}

#[test]
fn rejected_rename_keeps_the_existing_state() {
    let mut item = Item::new(Uuid::new_v4(), "original".into(), 1).unwrap();
    let original = item.clone();

    assert_eq!(item.rename(" \t\n".into()), Err(DomainError::EmptyTitle));
    assert_eq!(item, original);
}
