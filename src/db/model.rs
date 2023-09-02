use std::ops::Range;

use bson::Uuid as BsonUuid;

use chrono::NaiveDate;
use chrono::NaiveTime;
use chrono::Weekday;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Clone, Debug)]
pub struct Student {
	pub name: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum StudentStatus {
	Present,
	Pardoned,
	Missing,
}

#[derive(Serialize, Deserialize)]
pub struct BaseTimeSlot<UUID, Date> {
	pub user_id: String,
	pub id: UUID,
	pub subject: String,
	pub students: Vec<Student>,
	pub time: Range<NaiveTime>,
	pub timerange: Range<Date>,
	pub weekday: Weekday,
}

#[derive(Serialize, Deserialize, Debug)]
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
