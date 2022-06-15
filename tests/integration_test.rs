use rust_arango_trigger::{Trigger, TriggerConfig};

#[test]
fn setup_trigger() {
	let trigger = Trigger::new("http://localhost:8529", "triggers");

	trigger.subscribe(vec![
		TriggerConfig::new("users"),
		TriggerConfig::new_with_events("users2", vec!["insert/update"]),
		TriggerConfig::new_with_events_and_keys("users2", vec!["insert/update"], vec!["252525"]),
	]);
}
