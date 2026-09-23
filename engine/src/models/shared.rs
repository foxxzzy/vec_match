use serde::{Deserialize, Serialize};
use sqlx::{Decode, Encode};
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct PromptId(pub Uuid);

#[derive(Clone, Serialize, Deserialize, Debug, Encode, Decode)]
pub struct UserID(pub Uuid);

// this is the different ways in which a user can interact with a piece of content
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq, Hash)]
pub enum Reaction {
    Me,
    NotMe,
    Skip,
}

// these are the different content buckets, each one has its own collection in qdrant
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq, Hash)]
pub enum Axis {
    ComfortWithCloseness,
    NeedForReassurance,
    ConflictApproach,
    RepairStrategy,
    FutureFocus,
    RelationshipPace,
    MoralFraming,
    SocialValues,
    Interdependence,
    CommunicationStyle,
    ThinkingMode,
    SocialEnergy,
    RoutinePreference,
    RiskTolerance,
    ChangeOrientation,
    Playfulness,
}

// pub enum NewAxis {
//     ComfortWithCloseness,
//     NeedForReassurance,
//     ConflictApproach,
//     RepairStrategy,
//     FutureFocus,
//     RelationshipPace,
//     MoralFraming,
//     SocialValues,
//     Interdependence,
//     CommunicationStyle,
//     ThinkingMode,
//     SocialEnergy,
//     RoutinePreference,
//     RiskTolerance,
//     ChangeOrientation,
//     Playfulness,
// }

impl Axis {
    pub fn axis_to_text(&self) -> &'static str {
        match self {
            Axis::ComfortWithCloseness => "ComfortWithCloseness",
            Axis::NeedForReassurance => "NeedForReassurance",
            Axis::ConflictApproach => "ConflictApproach",
            Axis::RepairStrategy => "RepairStrategy",
            Axis::FutureFocus => "FutureFocus",
            Axis::RelationshipPace => "RelationshipPace",
            Axis::MoralFraming => "MoralFraming",
            Axis::SocialValues => "SocialValues",
            Axis::Interdependence => "Interdependence",
            Axis::CommunicationStyle => "CommunicationStyle",
            Axis::ThinkingMode => "ThinkingMode",
            Axis::SocialEnergy => "SocialEnergy",
            Axis::RoutinePreference => "RoutinePreference",
            Axis::RiskTolerance => "RiskTolerance",
            Axis::ChangeOrientation => "ChangeOrientation",
            Axis::Playfulness => "Playfulness",
        }
    }

    pub fn from_text(axis: &str) -> Result<Self, String> {
        match axis {
            "ComfortWithCloseness" => Ok(Axis::ComfortWithCloseness),
            "NeedForReassurance" => Ok(Axis::NeedForReassurance),
            "ConflictApproach" => Ok(Axis::ConflictApproach),
            "RepairStrategy" => Ok(Axis::RepairStrategy),
            "FutureFocus" => Ok(Axis::FutureFocus),
            "RelationshipPace" => Ok(Axis::RelationshipPace),
            "MoralFraming" => Ok(Axis::MoralFraming),
            "SocialValues" => Ok(Axis::SocialValues),
            "Interdependence" => Ok(Axis::Interdependence),
            "CommunicationStyle" => Ok(Axis::CommunicationStyle),
            "ThinkingMode" => Ok(Axis::ThinkingMode),
            "SocialEnergy" => Ok(Axis::SocialEnergy),
            "RoutinePreference" => Ok(Axis::RoutinePreference),
            "RiskTolerance" => Ok(Axis::RiskTolerance),
            "ChangeOrientation" => Ok(Axis::ChangeOrientation),
            "Playfulness" => Ok(Axis::Playfulness),
            other => Err(format!("unknown axis: {}", other)),
        }
    }

    pub fn decay_half_life_days(&self) -> f32 {
        match self {
            Axis::ComfortWithCloseness => DecayHalfLives::ComfortWithCloseness.value(),
            Axis::NeedForReassurance => DecayHalfLives::NeedForReassurance.value(),
            Axis::ConflictApproach => DecayHalfLives::ConflictApproach.value(),
            Axis::RepairStrategy => DecayHalfLives::RepairStrategy.value(),
            Axis::FutureFocus => DecayHalfLives::FutureFocus.value(),
            Axis::RelationshipPace => DecayHalfLives::RelationshipPace.value(),
            Axis::MoralFraming => DecayHalfLives::MoralFraming.value(),
            Axis::SocialValues => DecayHalfLives::SocialValues.value(),
            Axis::Interdependence => DecayHalfLives::Interdependence.value(),
            Axis::CommunicationStyle => DecayHalfLives::CommunicationStyle.value(),
            Axis::ThinkingMode => DecayHalfLives::ThinkingMode.value(),
            Axis::SocialEnergy => DecayHalfLives::SocialEnergy.value(),
            Axis::RoutinePreference => DecayHalfLives::RoutinePreference.value(),
            Axis::RiskTolerance => DecayHalfLives::RiskTolerance.value(),
            Axis::ChangeOrientation => DecayHalfLives::ChangeOrientation.value(),
            Axis::Playfulness => DecayHalfLives::Playfulness.value(),
        }
    }
}

// each piece of content is assigned into a bucket which should dictate how much it should scale the vector, (high quality content, more polarising content, more sway)
#[derive(Clone, Serialize, Deserialize)]
pub enum WeightBuckets {
    LOW,
    MEDIUM,
    HIGH,
}

impl WeightBuckets {
    pub fn value(&self) -> f32 {
        match self {
            WeightBuckets::LOW => 0.5,
            WeightBuckets::MEDIUM => 1.5,
            WeightBuckets::HIGH => 2.0,
        }
    }
}

pub enum DecayHalfLives {
    ComfortWithCloseness,
    NeedForReassurance,
    ConflictApproach,
    RepairStrategy,
    FutureFocus,
    RelationshipPace,
    MoralFraming,
    SocialValues,
    Interdependence,
    CommunicationStyle,
    ThinkingMode,
    SocialEnergy,
    RoutinePreference,
    RiskTolerance,
    ChangeOrientation,
    Playfulness,
}

impl DecayHalfLives {
    pub fn value(&self) -> f32 {
        match self {
            // NOTE: These are placeholder half-lives until calibrated per-axis.
            // They preserve the previous coarse categorization (slow/medium/fast).
            DecayHalfLives::ComfortWithCloseness => 270.0,
            DecayHalfLives::NeedForReassurance => 120.0,
            DecayHalfLives::ConflictApproach => 270.0,
            DecayHalfLives::RepairStrategy => 120.0,
            DecayHalfLives::FutureFocus => 540.0,
            DecayHalfLives::RelationshipPace => 60.0,
            DecayHalfLives::MoralFraming => 540.0,
            DecayHalfLives::SocialValues => 540.0,
            DecayHalfLives::Interdependence => 270.0,
            DecayHalfLives::CommunicationStyle => 270.0,
            DecayHalfLives::ThinkingMode => 540.0,
            DecayHalfLives::SocialEnergy => 270.0,
            DecayHalfLives::RoutinePreference => 540.0,
            DecayHalfLives::RiskTolerance => 270.0,
            DecayHalfLives::ChangeOrientation => 270.0,
            DecayHalfLives::Playfulness => 60.0,
        }
    }
}
