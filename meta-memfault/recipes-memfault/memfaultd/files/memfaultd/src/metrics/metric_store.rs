//
// Copyright (c) Memfault, Inc.
// See License.txt for details
use std::{collections::HashMap, path::Path};

use crate::{
    mar::{MarEntryBuilder, Metadata},
    metrics::{MetricReading, MetricStringKey, MetricValue},
};

use eyre::Result;

mod metric_aggregate;
use metric_aggregate::MetricAggregate;

use super::metric_reading::KeyedMetricReading;

pub struct InMemoryMetricStore {
    metrics: HashMap<MetricStringKey, MetricAggregate>,
}

impl InMemoryMetricStore {
    pub fn new() -> Self {
        InMemoryMetricStore {
            metrics: HashMap::new(),
        }
    }

    pub fn add_metric(&mut self, m: KeyedMetricReading) -> Result<()> {
        match self.metrics.entry(m.name) {
            std::collections::hash_map::Entry::Occupied(mut o) => {
                let state = o.get_mut();
                match (*state).aggregate(&m.value) {
                    Ok(s) => *state = s,
                    Err(e) => {
                        *state = MetricAggregate::new(&m.value)?;
                        log::warn!(
                            "New value for metric {} is incompatible ({}). Resetting metric.",
                            o.key(),
                            e
                        );
                    }
                }
            }
            std::collections::hash_map::Entry::Vacant(v) => {
                v.insert(MetricAggregate::new(&m.value)?);
            }
        };
        Ok(())
    }

    fn take_metrics(&mut self) -> HashMap<MetricStringKey, MetricValue> {
        std::mem::take(&mut self.metrics)
            .into_iter()
            .map(|(name, state)| (name, state.value()))
            .collect()
    }

    /// Create one heartbeat entry with all the metrics in the store.
    /// All data will be timestamped with current time measured by CollectionTime::now(), effectively
    /// disregarding the collectd timestamps.
    pub fn write_metrics(&mut self, mar_staging_area: &Path) -> Result<MarEntryBuilder<Metadata>> {
        Ok(
            MarEntryBuilder::new(mar_staging_area)?.set_metadata(Metadata::LinuxHeartbeat {
                metrics: self.take_metrics(),
            }),
        )
    }
}

impl Default for InMemoryMetricStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use tempfile::TempDir;

    use super::*;
    use crate::metrics::MetricTimestamp;
    use chrono::Duration;
    use std::str::FromStr;

    #[rstest]
    #[case(in_gauges(vec![("foo", 1.0), ("bar", 2.0), ("baz", 3.0)]), "foo", 1.0)]
    #[case(in_gauges(vec![("foo", 1.0), ("foo", 2.0), ("foo", 3.0)]), "foo", 3.0)]
    #[case(in_rates(vec![("foo", 1000, 1.0), ("foo", 1000, 1.0)]), "foo",  1.0)]
    #[case(in_rates(vec![("foo", 1000, 1.0), ("foo", 1000, 2.0)]), "foo", 1.5)]
    #[case(in_rates(vec![("foo", 1000, 1.0), ("foo", 1000, 2.0), ("foo", 1000, 2.0)]), "foo",  5.0/3.0)]
    fn test_aggregate_metrics(
        #[case] metrics: impl Iterator<Item = KeyedMetricReading>,
        #[case] name: &str,
        #[case] expected: f64,
    ) {
        let mut store = InMemoryMetricStore::new();

        for m in metrics {
            store.add_metric(m).unwrap();
        }
        let h = store.take_metrics();
        match h.get(&MetricStringKey::from_str(name).unwrap()).unwrap() {
            MetricValue::Number(e) => assert_eq!(*e, expected),
        }
    }

    #[rstest]
    fn test_empty_after_write() {
        let mut store = InMemoryMetricStore::new();
        for m in in_gauges(vec![("foo", 1.0), ("bar", 2.0), ("baz", 3.0)]) {
            store.add_metric(m).unwrap();
        }

        let tempdir = TempDir::new().unwrap();
        let _ = store.write_metrics(tempdir.path());
        assert_eq!(store.take_metrics().len(), 0);
    }

    fn in_gauges(metrics: Vec<(&'static str, f64)>) -> impl Iterator<Item = KeyedMetricReading> {
        metrics
            .into_iter()
            .enumerate()
            .map(|(i, (name, value))| KeyedMetricReading {
                name: MetricStringKey::from_str(name).unwrap(),
                value: MetricReading::Gauge {
                    value,
                    timestamp: MetricTimestamp::from_str("2021-01-01T00:00:00Z").unwrap()
                        + chrono::Duration::seconds(i as i64),
                },
            })
    }

    fn in_rates(
        metrics: Vec<(&'static str, i64, f64)>,
    ) -> impl Iterator<Item = KeyedMetricReading> {
        metrics
            .into_iter()
            .enumerate()
            .map(|(i, (name, interval, value))| KeyedMetricReading {
                name: MetricStringKey::from_str(name).unwrap(),
                value: MetricReading::Rate {
                    value,
                    interval: Duration::milliseconds(interval),
                    timestamp: MetricTimestamp::from_str("2021-01-01T00:00:00Z").unwrap()
                        + chrono::Duration::seconds(i as i64),
                },
            })
    }
}
