use std::collections::HashMap;

use itertools::Itertools;

use crate::db::model::{EntryState, Student, StudentStatus};

// TODO optimise this by writing to a single string instead of allocation like 9 bagillion strings
pub fn format_entry(entry: &EntryState, students: &[Student]) -> String {
	match entry.clone() {
		EntryState::Success { students } => {
			let mut status_map: HashMap<StudentStatus, Vec<Student>> = HashMap::new();

			for (student, status) in students {
				match status_map.get_mut(&status) {
					Some(s) => s.push(student),
					None => {
						status_map.insert(status, [student].into());
					}
				};
			}

			let present_students = status_map
				.get(&StudentStatus::Present)
				.map_or_else(|| "niemand".to_string(), |s| format_students(s));
			let pardoned_students = status_map
				.get(&StudentStatus::Pardoned)
				.map(|s| format_students(s));
			let missing_students = status_map
				.get(&StudentStatus::Missing)
				.map(|s| format_students(s));

			let base = format!(
				"Unterricht mit {present_students} hat planmäßig und erfolgreich stattgefunden."
			);

			let pardoned = if let Some(students) = pardoned_students {
				format!(" ({students} entschuldigt)")
			} else {
				String::new()
			};

			let missing = if let Some(students) = missing_students {
				format!(" ({students} fehlten unentschuldigt)")
			} else {
				String::new()
			};

			format!("{base}{pardoned}{missing}")
		}
		EntryState::CancelledByStudents => {
			let students = format_students(students);
			format!("Unterricht mit {students} wurde vom Matrosen abgesagt und nicht nachgeholt.")
		}
		EntryState::CancelledByTutor => {
			let students = format_students(students);
			format!("Unterricht mit {students} wurde von mir abgesagt und nicht nachgeholt.")
		}
		EntryState::Holidays => "Ferien".to_string(),
		EntryState::StudentsMissing => {
			let students = format_students(students);
			format!("Unterricht mit {students} konnte nicht stattfinden. Matrose(n) fehlte(n) unentschuldigt!")
		}
		EntryState::Other => {
			let students = format_students(students);
			format!("Unterricht mit {students} konnte aus unbekannten Gründen nicht stattfinden.")
		}
	}
}

fn format_students(students: &[Student]) -> String {
	students.iter().map(|v| &v.name).join(", ")
}
