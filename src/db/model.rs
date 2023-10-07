use std::fmt::Display;
use std::ops::Range;

use bson::Uuid as BsonUuid;

use chrono::NaiveDate;
use chrono::NaiveTime;
use chrono::Weekday;
use chrono_tz::Tz;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Clone, Debug)]
pub struct Student {
	pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq, Clone, Copy)]
#[repr(u8)]
pub enum StudentStatus {
	Present,
	Pardoned,
	Missing,
}

pub enum IntoEnumError {
	InvalidValue,
}

impl TryFrom<u8> for StudentStatus {
	type Error = IntoEnumError;

	fn try_from(value: u8) -> Result<Self, Self::Error> {
		match value {
			0 => Ok(StudentStatus::Present),
			1 => Ok(StudentStatus::Pardoned),
			2 => Ok(StudentStatus::Missing),
			_ => Err(IntoEnumError::InvalidValue)
		}
	}
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BaseTimeSlot<UUID, Date> {
	pub user_id: String,
	pub id: UUID,
	pub subject: String,
	pub students: Vec<Student>,
	pub time: Range<NaiveTime>,
	pub timerange: Range<Date>,
	/// FRONTEND ONLY DO NOT RELY ON.
	pub weekday: Weekday,
	pub timezone: Tz,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "status")]
pub enum EntryState {
	Success {
		students: Vec<(Student, StudentStatus)>,
	},
	CancelledByStudents,
	StudentsMissing,
	CancelledByTutor,
	Holidays,
	Other,
}

pub enum IntoEntryStateError {
	InvalidValue,
	MissingStudentsField,
}

impl TryFrom<(u8, Option<Vec<(Student, StudentStatus)>>)> for EntryState {
	type Error = IntoEntryStateError;

	fn try_from(value: (u8, Option<Vec<(Student, StudentStatus)>>)) -> Result<Self, Self::Error> {
		match value.0 {
			0 => {
				match value.1 {
					Some(students) => Ok(Self::Success { students }),
					None => Err(IntoEntryStateError::MissingStudentsField),
				}
			},
			1 => Ok(Self::CancelledByStudents),
			2 => Ok(Self::StudentsMissing),
			3 => Ok(Self::CancelledByTutor),
			4 => Ok(Self::Holidays),
			5 => Ok(Self::Other),
			_ => Err(IntoEntryStateError::InvalidValue),
		}
	}
}

impl Into<(u8, Option<Vec<(Student, StudentStatus)>>)> for EntryState {
	fn into(self) -> (u8, Option<Vec<(Student, StudentStatus)>>) {
		match self {
			Self::Success { students } => (0, Some(students)),
			Self::CancelledByStudents => (1, None),
			Self::StudentsMissing => (2, None),
			Self::CancelledByTutor => (3, None),
			Self::Holidays => (4, None),
			Self::Other => (5, None),
		}
	}
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BaseEntry<UUID> {
	pub user_id: String,
	pub index: u32,
	pub timeslot_id: UUID,
	pub state: EntryState,
}

pub type TimeSlot = BaseTimeSlot<uuid::Uuid, NaiveDate>;
pub type Entry = BaseEntry<uuid::Uuid>;

pub type BsonTimeSlot = BaseTimeSlot<BsonUuid, NaiveDate>;
pub type BsonEntry = BaseEntry<BsonUuid>;

impl From<BsonTimeSlot> for TimeSlot {
	fn from(v: BsonTimeSlot) -> Self {
		Self {
			user_id: v.user_id,
			id: v.id.into(),
			students: v.students,
			subject: v.subject,
			time: v.time,
			timerange: v.timerange,
			weekday: v.weekday,
			timezone: v.timezone,
		}
	}
}

impl From<BsonEntry> for Entry {
	fn from(v: BsonEntry) -> Self {
		Self {
			user_id: v.user_id,
			index: v.index,
			timeslot_id: v.timeslot_id.into(),
			state: v.state,
		}
	}
}

pub trait HasUserId {
	fn user_id(&self) -> &str;
	fn identifier(&self) -> String;
}

impl<U: Display, D> HasUserId for BaseTimeSlot<U, D> {
	fn user_id(&self) -> &str {
		&self.user_id
	}

	fn identifier(&self) -> String {
		format!("timeslot: {}", self.id)
	}
}

impl<U: Display> HasUserId for BaseEntry<U> {
	fn user_id(&self) -> &str {
		&self.user_id
	}

	fn identifier(&self) -> String {
		format!("entry: {}-{}", self.timeslot_id, self.index)
	}
}

#[derive(Serialize, Deserialize)]
pub struct ConfigurationEntry {
	pub key: String,
	pub value: String,
}
