use std::path::Path;

use num_bigint::BigUint;
use osv::{
    analysis::literals,
    source::{SourceFile, SourceId},
    syntax::{self, TokenKind},
    values::IntegerValue,
};

#[test]
fn expression_integer_tokens_decode_based_decimal_exponents_exactly() {
    let source = SourceFile::new(SourceId(0), Path::new("literal.vhd"), b"package values is constant a : integer := 1_000e2; constant b : integer := 16#ff#; constant c : integer := 2#1_000#e+2; end;");
    let parsed = syntax::parse(&source).unwrap();
    let actual = parsed
        .root()
        .all_tokens()
        .filter(|token| token.kind() == TokenKind::AbstractLiteral)
        .map(|token| literals::integer_magnitude(&token).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        actual,
        [
            BigUint::from(100_000u32),
            BigUint::from(255u32),
            BigUint::from(32u32)
        ]
    );
    let boundary = BigUint::from(1u8) << 63usize;
    assert_eq!(
        literals::checked_integer(boundary.clone(), true).unwrap(),
        IntegerValue(i64::MIN)
    );
    assert!(literals::checked_integer(boundary, false).is_err());
}
