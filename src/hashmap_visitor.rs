use std::collections::HashMap;

#[derive(Default)]
pub struct HashMapVisitor {
    fields: Option<HashMap<String, String>>,
}

impl HashMapVisitor {
    pub fn take(&mut self) -> HashMap<String, String> {
        self.fields.take().unwrap_or_default()
    }

    pub fn remove(&mut self, key: &str) -> Option<String> {
        if let Some(fields) = self.fields.as_mut() {
            fields.remove(key)
        } else {
            None
        }
    }

    fn add_value(&mut self, field: &tracing::field::Field, value: String) {
        if let Some(fields) = self.fields.as_mut() {
            fields.insert(field.name().to_owned(), value);
        } else {
            self.fields = Some(HashMap::from([(field.name().to_owned(), value)]));
        }
    }
}

impl tracing::field::Visit for HashMapVisitor {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.add_value(field, value.to_owned());
    }
    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        self.add_value(field, format!("{}", value));
    }
    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        self.add_value(field, format!("{}", value));
    }
    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.add_value(field, format!("{}", value));
    }
    fn record_debug(&mut self, _field: &tracing::field::Field, _value: &dyn std::fmt::Debug) {
        // Do nothing
    }
}
