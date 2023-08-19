use std::ops::Range;

use bson::DateTime as BsonDateTime;

use chrono::NaiveTime;
use chrono::Weekday;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
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
pub struct StudentState {
	pub student: Student,
	pub status: StudentStatus,
}

#[derive(Serialize, Deserialize)]
pub struct TimeSlot {
	pub students: Vec<Student>,
	pub weekday: Weekday,
	pub time: Range<NaiveTime>,
}

#[derive(Serialize, Deserialize)]
pub enum EntryState {
	Success { students: Vec<StudentState> },
	CanceledByStudents,
	CanceledByTutor,
}

#[derive(Serialize, Deserialize)]
pub struct Entry {
	pub ts: BsonDateTime,
	pub timeslot: TimeSlot,
	pub state: EntryState,
}
