use std::io::Cursor;

#[test]
fn setup_protocol_accepts_only_exact_commit_or_abort() {
    use mcsr_toolkit::setup_helper::Decision;
    assert_eq!(
        Decision::read(&mut Cursor::new(b"COMMIT\n")),
        Ok(Decision::Commit)
    );
    assert_eq!(
        Decision::read(&mut Cursor::new(b"ABORT\n")),
        Ok(Decision::Abort)
    );
    assert!(Decision::read(&mut Cursor::new(b"COMMIT extra\n")).is_err());
    assert!(Decision::read(&mut Cursor::new(Vec::<u8>::new())).is_err());
}
