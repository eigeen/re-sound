use std::io::{Read, Seek, Write};

use binrw::{BinRead, BinWrite, binrw};
use serde::{Deserialize, Serialize};

use crate::{bnk::BnkError, rwext::BinrwNullString};

use super::{EntryPayloadExt, Result, common::NodeBaseParams};

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct HircMusicSegment {
    music_segment_initial_values: MusicSegmentInitialValues,
}

impl EntryPayloadExt for HircMusicSegment {
    fn from_reader<R>(reader: &mut R, length: u32) -> Result<Self>
    where
        R: Read + Seek,
    {
        let start_pos = reader.stream_position()?;
        let music_segment_initial_values = MusicSegmentInitialValues::read(reader)?;
        let end_pos = reader.stream_position()?;
        let read_size = end_pos - start_pos;
        if read_size != length as u64 - 4 {
            return Err(BnkError::BadDataSize {
                name: "MusicSegment".to_string(),
                expected: length as u64 - 4,
                got: read_size,
                start: start_pos,
            });
        }
        Ok(HircMusicSegment {
            music_segment_initial_values,
        })
    }

    fn write_to<W>(&self, writer: &mut W) -> Result<()>
    where
        W: Write + Seek,
    {
        self.music_segment_initial_values.write(writer)?;
        Ok(())
    }
}

#[binrw]
#[brw(little)]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct MusicSegmentInitialValues {
    music_node_params: MusicNodeParams,
    duration: f64,
    num_markers: u32,
    #[br(count = num_markers)]
    markers: Vec<AkMusicMarkerWwise>,
}

#[binrw]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct MusicNodeParams {
    flags: u8,
    node_base_params: NodeBaseParams,
    children: Children,
    ak_meter_info: AkMeterInfo,
    meter_info_flag: u8,
    num_stingers: u32,
    #[br(count = num_stingers)]
    stingers: Vec<CAkStinger>,
}

#[binrw]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Children {
    num_children: u32,
    #[br(count = num_children)]
    children: Vec<u32>,
}

#[binrw]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct AkMeterInfo {
    grid_period: f64,
    grid_offset: f64,
    tempo: f32,
    time_sig_num_beats_bar: u8,
    time_sig_beat_value: u8,
}

#[binrw]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CAkStinger {
    trigger_id: u32,
    segment_id: u32,
    sync_play_at: u32,
    cue_filter_hash: u32,
    dont_repeat_time: i32,
    num_segment_look_ahead: u32,
}

#[binrw]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct AkMusicMarkerWwise {
    id: u32,
    position: f64,
    marker_name: BinrwNullString,
}
