use crate::artifacts::Identifier;

use super::{
    Effects, Folding, IntrinsicDefinition, IntrinsicId, IntrinsicName, NativeType, Operator,
    Purity, Signature, Specialization, Statement,
};

fn scalar(
    id: IntrinsicId,
    operator: Operator,
    arguments: &[NativeType],
    result: NativeType,
) -> IntrinsicDefinition {
    IntrinsicDefinition {
        id,
        signature: Signature {
            name: IntrinsicName::Operator(operator),
            arguments: arguments.to_vec(),
            result: Some(result),
        },
        purity: Purity::Pure,
        specialization: Specialization::Scalar,
        effects: Effects::None,
        folding: Folding::StaticArguments,
    }
}

pub(super) fn operations() -> Vec<IntrinsicDefinition> {
    use NativeType::{Boolean, Integer, Time};
    let mut definitions = vec![
        scalar(
            IntrinsicId::IntegerAdd,
            Operator::Add,
            &[Integer, Integer],
            Integer,
        ),
        scalar(
            IntrinsicId::IntegerSubtract,
            Operator::Subtract,
            &[Integer, Integer],
            Integer,
        ),
        scalar(
            IntrinsicId::IntegerMultiply,
            Operator::Multiply,
            &[Integer, Integer],
            Integer,
        ),
        scalar(
            IntrinsicId::IntegerDivide,
            Operator::Divide,
            &[Integer, Integer],
            Integer,
        ),
        scalar(
            IntrinsicId::IntegerNegate,
            Operator::Subtract,
            &[Integer],
            Integer,
        ),
        scalar(
            IntrinsicId::IntegerEqual,
            Operator::Equal,
            &[Integer, Integer],
            Boolean,
        ),
        scalar(
            IntrinsicId::IntegerNotEqual,
            Operator::NotEqual,
            &[Integer, Integer],
            Boolean,
        ),
        scalar(
            IntrinsicId::IntegerLess,
            Operator::Less,
            &[Integer, Integer],
            Boolean,
        ),
        scalar(
            IntrinsicId::IntegerLessEqual,
            Operator::LessEqual,
            &[Integer, Integer],
            Boolean,
        ),
        scalar(
            IntrinsicId::IntegerGreater,
            Operator::Greater,
            &[Integer, Integer],
            Boolean,
        ),
        scalar(
            IntrinsicId::IntegerGreaterEqual,
            Operator::GreaterEqual,
            &[Integer, Integer],
            Boolean,
        ),
        scalar(IntrinsicId::BooleanNot, Operator::Not, &[Boolean], Boolean),
        scalar(
            IntrinsicId::BooleanAnd,
            Operator::And,
            &[Boolean, Boolean],
            Boolean,
        ),
        scalar(
            IntrinsicId::BooleanOr,
            Operator::Or,
            &[Boolean, Boolean],
            Boolean,
        ),
        scalar(
            IntrinsicId::BooleanEqual,
            Operator::Equal,
            &[Boolean, Boolean],
            Boolean,
        ),
        scalar(
            IntrinsicId::BooleanNotEqual,
            Operator::NotEqual,
            &[Boolean, Boolean],
            Boolean,
        ),
        scalar(IntrinsicId::TimeAdd, Operator::Add, &[Time, Time], Time),
        scalar(
            IntrinsicId::TimeSubtract,
            Operator::Subtract,
            &[Time, Time],
            Time,
        ),
        scalar(
            IntrinsicId::TimeEqual,
            Operator::Equal,
            &[Time, Time],
            Boolean,
        ),
        scalar(
            IntrinsicId::TimeLess,
            Operator::Less,
            &[Time, Time],
            Boolean,
        ),
    ];
    for (id, statement, arguments, effects) in [
        (
            IntrinsicId::Report,
            Statement::Report,
            vec![NativeType::String, NativeType::Severity],
            Effects::Report,
        ),
        (
            IntrinsicId::Assert,
            Statement::Assert,
            vec![Boolean, NativeType::String, NativeType::Severity],
            Effects::Assert,
        ),
    ] {
        definitions.push(IntrinsicDefinition {
            id,
            signature: Signature {
                name: IntrinsicName::Statement(statement),
                arguments,
                result: None,
            },
            purity: Purity::Impure,
            specialization: Specialization::Diagnostic,
            effects,
            folding: Folding::Never,
        });
    }
    definitions.push(IntrinsicDefinition {
        id: IntrinsicId::Now,
        signature: Signature {
            name: IntrinsicName::Function(Identifier::Basic("now".into())),
            arguments: Vec::new(),
            result: Some(NativeType::DelayLength),
        },
        purity: Purity::Impure,
        specialization: Specialization::SimulationTime,
        effects: Effects::ObserveTime,
        folding: Folding::Never,
    });
    definitions
}
