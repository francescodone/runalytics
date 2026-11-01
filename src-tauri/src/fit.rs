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
    /// Barometric altitude in metres, when the device recorded it. Needed to
    /// price each kilometre split with its elevation gain.
    pub alt: Option<f64>,
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

/// Total ascent in metres from a Session's `total_ascent` plus its optional
/// `total_fractional_ascent` companion. The FIT profile stores the whole
/// metres in `total_ascent` (`Units: m`, no scale) and the centimetres in the
/// separate scale-100 fractional field; the old code divided the whole metres
/// by 10, so every climb decoded ten times too small. 0xFFFF is the invalid
/// sentinel, not a 6,553 m ascent, and a genuine zero (a pool swim) means
/// "not recorded" rather than a flat measurement.
fn ascent_m(total_ascent: u16, fractional_ascent: u8) -> Option<f64> {
    if total_ascent == 0 || total_ascent == u16::MAX {
        return None;
    }
    let fraction = if fractional_ascent == u8::MAX {
        0.0
    } else {
        fractional_ascent as f64 / 100.0
    };
    Some(total_ascent as f64 + fraction)
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
                    // A u32 field left at its invalid sentinel (0xFFFFFFFF)
                    // reads as a huge real value: a strength session's FIT
                    // once carried total_distance = 0xFFFFFFFF, which became a
                    // 42,949 km activity. Treat the sentinel as zero.
                    const INVALID32: u32 = 0xFFFF_FFFF;
                    out.duration_s = if ses.total_timer_time == INVALID32 {
                        0.0
                    } else {
                        ses.total_timer_time as f64 / 1000.0
                    };
                    out.distance_m = if ses.total_distance == INVALID32 {
                        0.0
                    } else {
                        ses.total_distance as f64 / 100.0
                    };
                    if ses.avg_heart_rate > 0 {
                        out.avg_hr = Some(ses.avg_heart_rate as i64);
                    }
                    if ses.max_heart_rate > 0 {
                        out.max_hr = Some(ses.max_heart_rate as i64);
                    }
                    out.elevation_gain_m =
                        ascent_m(ses.total_ascent, ses.total_fractional_ascent);
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
                    // Altitude is scale 5 / offset 500 in both the legacy and
                    // enhanced fields; the enhanced field wins when present.
                    // A u16::MAX/u32::MAX field is the invalid sentinel.
                    let alt = if rec.enhanced_altitude != u32::MAX {
                        Some(rec.enhanced_altitude as f64 / 5.0 - 500.0)
                    } else if rec.altitude != u16::MAX {
                        Some(rec.altitude as f64 / 5.0 - 500.0)
                    } else {
                        None
                    };
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
                                alt,
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

#[cfg(test)]
mod tests {
    use super::ascent_m;

    /// The FIT profile stores Session.total_ascent in whole metres — the
    /// scale-100 field is the separate fractional companion. The shipped
    /// code divided by 10, so a 450 m climb decoded as 45 m. This pins the
    /// magnitude, not just the round-trip.
    #[test]
    fn ascent_is_in_metres_not_decimetres() {
        assert_eq!(ascent_m(450, u8::MAX), Some(450.0));
        assert_eq!(ascent_m(450, 57), Some(450.57));
    }

    /// 0xFFFF is the FIT invalid sentinel and a genuine zero is "no ascent
    /// recorded"; neither may become a number the UI prints as metres.
    #[test]
    fn ascent_rejects_sentinel_and_zero() {
        assert_eq!(ascent_m(u16::MAX, 0), None);
        assert_eq!(ascent_m(0, 0), None);
    }
}
