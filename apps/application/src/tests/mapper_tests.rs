use uuid::Uuid;

use crate::{
    CreateItemCommand,
    Error,
    mappers::item::{
        from_create,
        to_create,
    },
};

#[test]
fn create_mapping_requires_identity_and_time_and_preserves_domain_validation() {
    let command = CreateItemCommand {
        title: " résumé 🦀 ".into(),
    };
    let item = from_create(command, Uuid::nil(), 42).unwrap();
    assert_eq!(item.id(), Uuid::nil());
    assert_eq!(item.created_at(), 42);
    assert_eq!(item.title(), "résumé 🦀");
    assert_eq!(
        from_create(to_create(&item), item.id(), item.created_at()).unwrap(),
        item
    );
    let invalid = CreateItemCommand {
        title: " \t".into(),
    };
    assert_eq!(
        from_create(invalid, Uuid::nil(), 42),
        Err(Error::EmptyTitle)
    );
}
