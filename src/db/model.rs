use std::ops::Range;
use std::str::FromStr;

use chrono::Datelike;
use chrono::NaiveDate;
use chrono::NaiveTime;
use chrono::Weekday;
use chrono_tz::Tz;

use serde::{Deserialize, Serialize};
use sqlx::prelude::Type;
use tracing::error;

use uuid::Uuid;

#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Clone, Debug)]
pub struct Student {
	pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq, Clone, Copy, Type)]
#[sqlx(type_name = "student_status")]
#[sqlx(rename_all = "lowercase")]
pub enum StudentStatus {
	Present,
	Pardoned,
	Missing,
}

#[derive(Serialize, Deserialize, Debug, Type)]
#[sqlx(type_name = "student_status")]
pub struct StudentState {
	pub student: String,
	pub status: StudentStatus,
}

impl sqlx::postgres::PgHasArrayType for StudentState {
	fn array_type_info() -> sqlx::postgres::PgTypeInfo {
		StudentState::type_info()
	}
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
			_ => Err(IntoEnumError::InvalidValue),
		}
	}
}

#[derive(Debug, sqlx::Type)]
#[sqlx(type_name = "timeslot_time")]
pub struct DbTime {
	pub beginning: NaiveTime,
	pub finish: NaiveTime,
}

#[derive(Debug, sqlx::Type)]
#[sqlx(type_name = "timeslot_range")]
pub struct DbTimerange {
	pub beginning: NaiveDate,
	pub finish: NaiveDate,
}

#[derive(Debug)]
pub struct TimeSlot {
	pub user_id: String,
	pub id: Uuid,
	pub subject: String,
	pub students: Vec<String>,
	pub time: DbTime,
	pub timerange: DbTimerange,
	pub timezone: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Type)]
#[sqlx(type_name = "entry_state")]
#[sqlx(rename_all = "lowercase")]
pub enum EntryState {
	Success,
	CancelledByStudents,
	StudentsMissing,
	CancelledByTutor,
	Holidays,
	Other,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "status")]
pub enum OldEntryState {
	Success {
		students: Vec<(Student, StudentStatus)>,
	},
	CancelledByStudents,
	StudentsMissing,
	CancelledByTutor,
	Holidays,
	Other,
	InvalidData,
}

#[derive(Debug)]
pub struct Entry {
	pub user_id: String,
	pub index: i32,
	pub timeslot_id: Uuid,
	pub state: Option<sqlx::types::Json<OldEntryState>>,
	pub state_enum: Option<EntryState>,
	pub students: Option<Vec<StudentState>>,
}

#[derive(Serialize, Deserialize)]
pub struct WebTimeSlot {
	pub user_id: String,
	pub id: Uuid,
	pub subject: String,
	pub students: Vec<String>,
	pub time: Range<NaiveTime>,
	pub timerange: Range<NaiveDate>,
	pub weekday: Weekday,
	pub timezone: Tz,
}

pub fn convert_ts(ts: TimeSlot) -> Option<WebTimeSlot> {
	let timezone: chrono_tz::Tz = match chrono_tz::Tz::from_str(&ts.timezone) {
		Ok(tz) => tz,
		Err(e) => {
			error!(%e, "invalid timezone data in db");
			return None;
		}
	};

	let time = ts.time.beginning..ts.time.finish;
	let timerange = ts.timerange.beginning..ts.timerange.finish;

	let weekday = ts.timerange.beginning.weekday();

	Some(WebTimeSlot {
		user_id: ts.user_id,
		id: ts.id,
		subject: ts.subject,
		students: ts.students,
		time,
		timerange,
		weekday,
		timezone,
	})
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WebEntry {
	pub user_id: String,
	pub index: u32,
	pub timeslot_id: Uuid,
	pub state: EntryState,
	pub students: Vec<StudentState>,
}

pub fn convert_entry(e: Entry) -> Option<WebEntry> {
	let index: u32 = match e.index.try_into() {
		Ok(i) => i,
		Err(e) => {
			error!(%e, "invalid index data in db");
			return None;
		}
	};

	Some(WebEntry {
		user_id: e.user_id,
		index,
		timeslot_id: e.timeslot_id,
		state: e.state_enum.unwrap(),
		students: e.students.unwrap(),
	})
}

pub trait HasUserId {
	fn user_id(&self) -> &str;
	fn identifier(&self) -> String;
}

impl HasUserId for WebTimeSlot {
	fn user_id(&self) -> &str {
		&self.user_id
	}

	fn identifier(&self) -> String {
		format!("timeslot: {}", self.id)
	}
}

impl HasUserId for WebEntry {
	fn user_id(&self) -> &str {
		&self.user_id
	}

	fn identifier(&self) -> String {
		format!("entry: {}-{}", self.timeslot_id, self.index)
	}
}
