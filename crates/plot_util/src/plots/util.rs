use chrono::{DateTime, Utc};

pub(crate) fn offset_data_iter<'i>(
    mut data_iter: impl Iterator<Item = &'i mut [f64; 2]>,
    new_start_date: DateTime<Utc>,
) {
    if let Some(first_point) = data_iter.next() {
        let new_date_ns = new_start_date
            .timestamp_nanos_opt()
            .expect("Nanoseconds overflow") as f64;
        let offset = new_date_ns - first_point[0];
        // Remember to also offset the first point that has been removed from the iterator!
        first_point[0] += offset;
        for point in data_iter {
            point[0] += offset;
        }
    }
}
