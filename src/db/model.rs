use std::ops::Range;

use bson::DateTime as BsonDateTime;
use bson::Uuid as BsonUuid;

use chrono::{DateTime, Utc};
use chrono::NaiveTime;
use chrono::Weekday;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub struct Student {
	pub name: String,
}

#[derive(Serialize, Deserialize)]
pub enum StudentStatus {
	Present,
	Pardoned,
	Missing,
}

#[derive(Serialize, Deserialize)]
pub struct BaseTimeSlot<UUID, Date> {
	pub id: UUID,
	pub students: Vec<Student>,
	pub weekday: Weekday,
	pub time: Range<NaiveTime>,
	pub timerange: Range<Date>,
}

#[derive(Serialize, Deserialize)]
pub enum EntryState {
	Success { students: Vec<(Student, StudentStatus)> },
	CanceledByStudents,
	CanceledByTutor,
	Holidays,
	Other,
}

#[derive(Serialize, Deserialize)]
pub struct BaseEntry<UUID, Date> {
	pub index: u32,
	pub timeslot: BaseTimeSlot<UUID, Date>,
	pub state: EntryState,
}

pub type TimeSlot = BaseTimeSlot<uuid::Uuid, DateTime<Utc>>;
pub type Entry = BaseEntry<uuid::Uuid, DateTime<Utc>>;

pub type BsonTimeSlot = BaseTimeSlot<BsonUuid, BsonDateTime>;
pub type BsonEntry = BaseEntry<BsonUuid, BsonDateTime>;

impl From<BsonTimeSlot> for TimeSlot {
	fn from(v: BsonTimeSlot) -> Self {
		Self {
			id: v.id.into(),
			students: v.students,
			weekday: v.weekday,
			time: v.time,
			timerange: Range { start: v.timerange.start.into(), end: v.timerange.end.into() },
		}
	}
}

impl From<BsonEntry> for Entry {
	fn from(v: BsonEntry) -> Self {
		Self {
			index: v.index,
			timeslot: v.timeslot.into(),
			state: v.state,
		}
	}
}
