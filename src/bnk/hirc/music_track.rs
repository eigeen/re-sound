use std::io::{Read, Seek, Write};

use binrw::{BinRead, BinWrite, binrw};
use serde::{Deserialize, Serialize};

use crate::bnk::BnkError;

use super::{
    EntryPayloadExt, Result,
    common::{AkRTPCGraphPoint, NodeBaseParams},
};

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct HircMusicTrack {
    music_track_initial_values: MusicTrackInitialValues,
}

impl EntryPayloadExt for HircMusicTrack {
    fn from_reader<R>(reader: &mut R, length: u32) -> Result<Self>
    where
        R: Read + Seek,
    {
        let pos_start = reader.stream_position()?;
        let music_track_initial_values = MusicTrackInitialValues::read(reader)?;
        let pos_end = reader.stream_position()?;
        let read_size = pos_end - pos_start;
        if read_size != length as u64 - 4 {
            return Err(BnkError::BadDataSize {
                name: "MusicTrackInitialValues".to_string(),
                expected: length as u64 - 4,
                got: read_size,
                start: pos_start,
            });
        }
        Ok(HircMusicTrack {
            music_track_initial_values,
        })
    }

    fn write_to<W>(&self, writer: &mut W) -> Result<()>
    where
        W: Write + Seek,
    {
        self.music_track_initial_values.write(writer)?;
        Ok(())
    }
}

#[binrw]
#[brw(little)]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct MusicTrackInitialValues {
    flags: u8,
    num_sources: u32,
    #[br(count = num_sources)]
    sources: Vec<AkBankSourceData>,
    num_playlist_items: u32,
    #[br(count = num_playlist_items)]
    playlist: Vec<AkTrackSrcInfo>,
    #[br(if(num_playlist_items > 0))]
    #[bw(if(*num_playlist_items > 0))]
    num_sub_track: u32,
    num_clip_automations: u32,
    #[br(count = num_clip_automations)]
    clip_automations: Vec<AkClipAutomation>,
    node_base_params: NodeBaseParams,
    track_type: AkMusicTrackType,
    #[br(if(track_type == AkMusicTrackType::Switch))]
    #[bw(if(*track_type == AkMusicTrackType::Switch))]
    switch_params: Option<SwitchParams>,
    #[br(if(track_type == AkMusicTrackType::Switch))]
    #[bw(if(*track_type == AkMusicTrackType::Switch))]
    trans_params: Option<TransParams>,
    look_ahead_time: i32,
}

#[binrw]
#[brw(little)]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct AkBankSourceData {
    plugin_id: u32,
    stream_type: u8,
    media_information: AkMediaInformation,
}

#[binrw]
#[brw(little)]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct AkMediaInformation {
    source_id: u32,
    in_memory_media_size: u32,
    source_bits: u8,
}

#[binrw]
#[brw(little)]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct AkTrackSrcInfo {
    track_id: u32,
    source_id: u32,
    event_id: u32,
    play_at: f64,
    begin_trim_offset: f64,
    end_trim_offset: f64,
    src_duration: f64,
}

#[binrw]
#[brw(little)]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct AkClipAutomation {
    clip_index: u32,
    auto_type: u32,
    graph_points_count: u32,
    #[br(count = graph_points_count)]
    graph_points: Vec<AkRTPCGraphPoint>,
}

#[repr(u8)]
#[binrw]
#[brw(repr(u8))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum AkMusicTrackType {
    Normal = 0x0,
    Random = 0x1,
    Sequence = 0x2,
    Switch = 0x3,
}

#[binrw]
#[brw(little)]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SwitchParams {
    group_type: u8,
    group_id: u32,
    default_switch: u32,
    num_switch_assoc: u32,
    #[br(count = num_switch_assoc)]
    switch_assoc: Vec<TrackSwitchAssoc>,
}

#[binrw]
#[brw(little)]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct TrackSwitchAssoc {
    switch_assoc: u32,
}

#[binrw]
#[brw(little)]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct TransParams {
    src_fade_params: FadeParams,
    sync_type: u32,
    cue_filter_hash: u32,
    dest_fade_params: FadeParams,
}

#[binrw]
#[brw(little)]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FadeParams {
    transition_time: i32,
    fade_curve: u32,
    fade_offset: i32,
}
