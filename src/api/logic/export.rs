use std::collections::HashMap;

use itertools::Itertools;

use crate::db::model::{EntryState, StudentState, StudentStatus};

fn seperate_status_map(students: &[StudentState]) -> HashMap<StudentStatus, Vec<String>> {
	// TODO increase if we add too many StudentStatuses
	let mut status_map: HashMap<StudentStatus, Vec<String>> = HashMap::with_capacity(16);

	for StudentState { student, status } in students {
		if let Some(s) = status_map.get_mut(status) {
			s.push(student.clone());
		} else {
			let mut s = Vec::with_capacity(16);

			s.push(student.clone());

			status_map.insert(*status, s);
		}
	}

	status_map
}

// TODO optimise this by writing to a single string instead of allocation like 9 bagillion strings
pub fn format_entry(
	state_enum: EntryState,
	students: &[StudentState],
	timeslot_students: &[String],
) -> String {
	let all_students = format_students(timeslot_students);

	let status_map = seperate_status_map(students);

	let present_students = status_map
		.get(&StudentStatus::Present)
		.map(|s| format_students(s));
	let pardoned_students = status_map
		.get(&StudentStatus::Pardoned)
		.map(|s| format_students(s));
	let missing_students = status_map
		.get(&StudentStatus::Missing)
		.map(|s| format_students(s));

	let base =
			match state_enum {
				EntryState::Success => format!("Unterricht mit {} hat planmäßig und erfolgreich stattgefunden.", present_students.as_ref().unwrap_or(&all_students)),
				EntryState::CancelledByStudents => format!("Unterricht mit {} wurde vom Matrosen abgesagt und nicht nachgeholt.", pardoned_students.as_ref().unwrap()),
				EntryState::CancelledByTutor => format!("Unterricht mit {all_students} wurde von mir abgesagt und nicht nachgeholt."),
				EntryState::StudentsMissing => format!("Unterricht mit {all_students} konnte nicht stattfinden. Matrose(n) fehlte(n) unentschuldigt!"),
				EntryState::Holidays => "Ferien".to_string(),
				EntryState::Other => format!("Unterricht mit {all_students} konnte aus unbekannten Gründen nicht stattfinden."),
			};

	let pardoned = if let Some(students) = pardoned_students.as_ref() {
		format!(" ({students} entschuldigt)")
	} else {
		String::new()
	};

	let missing = if let Some(students) = missing_students {
		format!(" ({students} fehlte(n) unentschuldigt)")
	} else {
		String::new()
	};

	format!("{base}{pardoned}{missing}")
}

fn format_students(students: &[String]) -> String {
	students.iter().join(", ")
}
