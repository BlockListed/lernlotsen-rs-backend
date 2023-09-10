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
pub enum StudentStatus {
	Present,
	Pardoned,
	Missing,
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
