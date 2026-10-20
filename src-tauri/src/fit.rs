//! FIT file decoding via rustyfit. Produces a `DecodedActivity` with summary
//! fields from the SESSION message and per-second track points from RECORD
//! messages (position in semicircles → degrees via rustyfit::semconv).

use rustyfit::profile::{mesgdef, typedef};
use rustyfit::semconv::to_degrees;
use rustyfit::{Decoder, DecoderEvent, StreamingIterator};

use crate::error::{AppError, Result};

#[derive(Debug, Clone)]
pub struct TrackPoint {
    pub ts: i64,
    pub lat: f64,
    pub lon: f64,
    pub hr: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct DecodedActivity {
    pub record_id: String,
    pub sport: String,
    /// The provider's own numeric sport code (COROS `SportType`), needed to
    /// fetch this activity's FIT file later. None for providers that don't
    /// have one, or when a FIT session's enum didn't map to a COROS code.
    pub provider_sport_code: Option<i64>,
    pub start_ts: i64,
    pub duration_s: f64,
    pub distance_m: f64,
    pub avg_hr: Option<i64>,
    pub max_hr: Option<i64>,
    pub elevation_gain_m: Option<f64>,
    pub points: Vec<TrackPoint>,
}

/// FIT epoch offset: 1989-12-31 00:00:00 UTC.
const FIT_EPOCH_OFFSET: i64 = 631_065_600;

fn fit_ts(dt: typedef::DateTime) -> i64 {
    let raw = dt.0 as i64;
    if raw == 0 || raw >= 0xFFFF_FFFF {
        0
    } else {
        raw + FIT_EPOCH_OFFSET
    }
}

pub fn decode_fit(record_id: &str, bytes: &[u8]) -> Result<DecodedActivity> {
    let mut dec = Decoder::new();
    let mut reader = embedded_io_adapters::std::FromStd::new(bytes);
    let mut stream = dec.stream(&mut reader);

    let mut out = DecodedActivity {
        record_id: record_id.to_string(),
        sport: "run".into(),
        provider_sport_code: None,
        start_ts: 0,
        duration_s: 0.0,
        distance_m: 0.0,
        avg_hr: None,
        max_hr: None,
        elevation_gain_m: None,
        points: Vec::new(),
    };
    let mut last_ts = 0i64;

    while let Some(event) = stream.next() {
        let event = event.map_err(|e| AppError::Other(format!("fit decode: {e}")))?;
        match event {
            DecoderEvent::Message(mesg) => match mesg.num {
                typedef::MesgNum::SESSION => {
                    let ses = mesgdef::Session::from(mesg);
                    out.sport = ses.sport.0.to_string();
                    out.start_ts = fit_ts(ses.start_time);
                    out.duration_s = ses.total_timer_time as f64 / 1000.0;
                    out.distance_m = ses.total_distance as f64 / 100.0;
                    if ses.avg_heart_rate > 0 {
                        out.avg_hr = Some(ses.avg_heart_rate as i64);
                    }
                    if ses.max_heart_rate > 0 {
                        out.max_hr = Some(ses.max_heart_rate as i64);
                    }
                    if ses.total_ascent > 0 {
                        out.elevation_gain_m = Some(ses.total_ascent as f64 / 10.0);
                    }
                }
                typedef::MesgNum::RECORD => {
                    let rec = mesgdef::Record::from(mesg);
                    let ts = fit_ts(rec.timestamp);
                    if ts != 0 {
                        last_ts = ts;
                    }
                    // position_lat/long are semicircles (i32); to_degrees
                    // returns None for the invalid sentinel.
                    let lat = to_degrees(rec.position_lat);
                    let lon = to_degrees(rec.position_long);
                    if let (Some(lat), Some(lon)) = (lat, lon) {
                        if lat.abs() <= 90.0 && lon.abs() <= 180.0 && (lat != 0.0 || lon != 0.0) {
                            out.points.push(TrackPoint {
                                ts: last_ts,
                                lat,
                                lon,
                                hr: if rec.heart_rate > 0 {
                                    Some(rec.heart_rate as i64)
                                } else {
                                    None
                                },
                            });
                        }
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }
    Ok(out)
}
