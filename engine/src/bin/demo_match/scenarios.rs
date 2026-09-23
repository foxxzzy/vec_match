use matching_engine_demo::models::{Axis, Reaction};

#[derive(Clone, Copy)]
pub(crate) enum GateCase {
    Normal,
    AgeMismatch,
    DistanceMismatch,
    HeightMismatch,
    GenderMismatch,
    SmokingMismatch,
}

#[derive(Clone, Copy)]
pub(crate) enum PersonalityCase {
    None,
    Same,
    OppositeAll,
    MajorConflict,
    LowWeightDifferences,
}

#[derive(Clone, Copy)]
pub(crate) enum WorkflowCase {
    None,
    ConfirmMatch,
    FinalReject,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExpectedStage {
    HardGateFail,
    EngineFail,
    EnginePass,
    MatchCreated,
    FinalRejected,
}

pub(crate) struct ScenarioSpec {
    pub(crate) number: usize,
    pub(crate) a_name: &'static str,
    pub(crate) b_name: &'static str,
    pub(crate) title: &'static str,
    pub(crate) explanation: &'static str,
    pub(crate) gate_case: GateCase,
    pub(crate) personality_case: PersonalityCase,
    pub(crate) workflow_case: WorkflowCase,
    pub(crate) expected: ExpectedStage,
}

pub(crate) fn scenarios() -> Vec<ScenarioSpec> {
    vec![
        ScenarioSpec {
            number: 1,
            a_name: "Alice",
            b_name: "Bob",
            title: "Compatible pair reaches a confirmed match",
            explanation: "Their profiles fit, their synthetic personality interactions are aligned, both like each other, and both final decisions are yes.",
            gate_case: GateCase::Normal,
            personality_case: PersonalityCase::Same,
            workflow_case: WorkflowCase::ConfirmMatch,
            expected: ExpectedStage::MatchCreated,
        },
        ScenarioSpec {
            number: 2,
            a_name: "Chloe",
            b_name: "Daniel",
            title: "Age preference rejects the pair",
            explanation: "Chloe accepts ages 24 to 30. Daniel is deliberately outside that range.",
            gate_case: GateCase::AgeMismatch,
            personality_case: PersonalityCase::None,
            workflow_case: WorkflowCase::None,
            expected: ExpectedStage::HardGateFail,
        },
        ScenarioSpec {
            number: 3,
            a_name: "Eva",
            b_name: "Finn",
            title: "Distance preference rejects the pair",
            explanation: "Eva is placed in London, Finn in Edinburgh, and both have a 25 km maximum distance.",
            gate_case: GateCase::DistanceMismatch,
            personality_case: PersonalityCase::None,
            workflow_case: WorkflowCase::None,
            expected: ExpectedStage::HardGateFail,
        },
        ScenarioSpec {
            number: 4,
            a_name: "Grace",
            b_name: "Henry",
            title: "Height preference rejects the pair",
            explanation: "Grace has a maximum preferred height of 175 cm. Henry is deliberately 195 cm.",
            gate_case: GateCase::HeightMismatch,
            personality_case: PersonalityCase::None,
            workflow_case: WorkflowCase::None,
            expected: ExpectedStage::HardGateFail,
        },
        ScenarioSpec {
            number: 5,
            a_name: "Iris",
            b_name: "Jack",
            title: "Gender preference rejects the pair",
            explanation: "Iris is deliberately configured not to include Jack's gender code in her preferred genders.",
            gate_case: GateCase::GenderMismatch,
            personality_case: PersonalityCase::None,
            workflow_case: WorkflowCase::None,
            expected: ExpectedStage::HardGateFail,
        },
        ScenarioSpec {
            number: 6,
            a_name: "Kara",
            b_name: "Liam",
            title: "Smoking preference rejects the pair",
            explanation: "Kara excludes smokers. Liam is deliberately assigned the database's smoker status.",
            gate_case: GateCase::SmokingMismatch,
            personality_case: PersonalityCase::None,
            workflow_case: WorkflowCase::None,
            expected: ExpectedStage::HardGateFail,
        },
        ScenarioSpec {
            number: 7,
            a_name: "Maya",
            b_name: "Noah",
            title: "Profiles fit but personality vectors are opposite",
            explanation: "Maya reacts Me to the seeded prompts while Noah reacts Not Me to the same prompts. The database hard gate should pass, then the personality engine should reject them.",
            gate_case: GateCase::Normal,
            personality_case: PersonalityCase::OppositeAll,
            workflow_case: WorkflowCase::None,
            expected: ExpectedStage::EngineFail,
        },
        ScenarioSpec {
            number: 8,
            a_name: "Olivia",
            b_name: "Peter",
            title: "Major values conflicts outweigh other similarities",
            explanation: "They deliberately disagree on heavily weighted areas such as future focus, moral framing, social values, communication, thinking style, relationship pace, conflict and repair.",
            gate_case: GateCase::Normal,
            personality_case: PersonalityCase::MajorConflict,
            workflow_case: WorkflowCase::None,
            expected: ExpectedStage::EngineFail,
        },
        ScenarioSpec {
            number: 9,
            a_name: "Quinn",
            b_name: "Ryan",
            title: "Lower-weight differences are tolerated",
            explanation: "They differ on playfulness, risk tolerance, routine preference and social energy, while the more important axes remain aligned.",
            gate_case: GateCase::Normal,
            personality_case: PersonalityCase::LowWeightDifferences,
            workflow_case: WorkflowCase::None,
            expected: ExpectedStage::EnginePass,
        },
        ScenarioSpec {
            number: 10,
            a_name: "Sophie",
            b_name: "Theo",
            title: "Compatibility never overrides the final human decision",
            explanation: "They pass the hard gate and score highly. Both initially like each other, but Theo says no at the final decision stage.",
            gate_case: GateCase::Normal,
            personality_case: PersonalityCase::Same,
            workflow_case: WorkflowCase::FinalReject,
            expected: ExpectedStage::FinalRejected,
        },
    ]
}


pub(crate) fn reaction_for_case(personality_case: PersonalityCase, axis: &Axis) -> Reaction {
    match personality_case {
        PersonalityCase::None | PersonalityCase::Same => Reaction::Me,
        PersonalityCase::OppositeAll => Reaction::NotMe,
        PersonalityCase::MajorConflict => {
            if matches!(
                axis,
                Axis::FutureFocus
                    | Axis::MoralFraming
                    | Axis::SocialValues
                    | Axis::CommunicationStyle
                    | Axis::ThinkingMode
                    | Axis::RelationshipPace
                    | Axis::ConflictApproach
                    | Axis::RepairStrategy
            ) {
                Reaction::NotMe
            } else {
                Reaction::Me
            }
        }
        PersonalityCase::LowWeightDifferences => {
            if matches!(
                axis,
                Axis::Playfulness
                    | Axis::RiskTolerance
                    | Axis::RoutinePreference
                    | Axis::SocialEnergy
            ) {
                Reaction::NotMe
            } else {
                Reaction::Me
            }
        }
    }
}

