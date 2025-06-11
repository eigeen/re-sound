use std::io::{Read, Seek, Write};

use binrw::{BinRead, BinWrite, binrw};
use serde::{Deserialize, Serialize};

use crate::bnk::BnkError;

use super::{EntryPayloadExt, MusicNodeParams, Result};

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct HircMusicRanSecCntr {
    music_ran_sec_cntr_initial_values: MusicRanSecCntrInitialValues,
}

impl EntryPayloadExt for HircMusicRanSecCntr {
    fn from_reader<R>(reader: &mut R, length: u32) -> Result<Self>
    where
        R: Read + Seek,
    {
        let pos_start = reader.stream_position()?;
        let music_ran_sec_cntr_initial_values = MusicRanSecCntrInitialValues::read_le(reader)?;
        let pos_end = reader.stream_position()?;
        let read_size = pos_end - pos_start;
        if read_size != length as u64 - 4 {
            return Err(BnkError::BadDataSize {
                name: "MusicRanSecCntrInitialValues".to_string(),
                expected: length as u64 - 4,
                got: read_size,
                start: pos_start,
            });
        }
        Ok(HircMusicRanSecCntr {
            music_ran_sec_cntr_initial_values,
        })
    }

    fn write_to<W>(&self, writer: &mut W) -> Result<()>
    where
        W: Write + Seek,
    {
        self.music_ran_sec_cntr_initial_values.write_le(writer)?;
        Ok(())
    }

    fn fix_values(&mut self) -> Result<()> {
        // fix num_play_list_items
        let num_play_list_items = self
            .music_ran_sec_cntr_initial_values
            .play_list_items
            .iter()
            .map(get_num_recursive)
            .sum::<u32>();
        self.music_ran_sec_cntr_initial_values.num_play_list_items = num_play_list_items;

        Ok(())
    }
}

fn get_num_recursive(play_list_item: &AkMusicRanSeqPlaylistItem) -> u32 {
    let mut num = 1; // Count this item
    for child in &play_list_item.play_list {
        num += get_num_recursive(child); // Recursively count children
    }
    num
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct MusicRanSecCntrInitialValues {
    music_trans_node_params: MusicTransNodeParams,
    /// This is total number of play list items, recursive.
    num_play_list_items: u32,
    play_list_items: Vec<AkMusicRanSeqPlaylistItem>,
}

impl BinRead for MusicRanSecCntrInitialValues {
    type Args<'a> = ();

    fn read_options<R: Read + Seek>(
        reader: &mut R,
        _endian: binrw::Endian,
        args: Self::Args<'_>,
    ) -> binrw::BinResult<Self> {
        let music_trans_node_params = MusicTransNodeParams::read_args(reader, args)?;
        let num_play_list_items = u32::read_le(reader)?;

        let mut play_list_items = Vec::with_capacity(1);
        loop {
            let play_list_item = AkMusicRanSeqPlaylistItem::read_args(reader, args)?;
            let num = get_num_recursive(&play_list_item);
            play_list_items.push(play_list_item);

            match num.cmp(&num_play_list_items) {
                // num < num_play_list_items, keep reading
                std::cmp::Ordering::Less => continue,
                std::cmp::Ordering::Equal => break,
                std::cmp::Ordering::Greater => {
                    return Err(binrw::Error::AssertFail {
                        pos: reader.stream_position()?,
                        message: " num_play_list_items is larger than expected".to_string(),
                    });
                }
            }
        }

        Ok(MusicRanSecCntrInitialValues {
            music_trans_node_params,
            num_play_list_items,
            play_list_items,
        })
    }
}

impl BinWrite for MusicRanSecCntrInitialValues {
    type Args<'a> = ();

    fn write_options<W: Write + Seek>(
        &self,
        writer: &mut W,
        _endian: binrw::Endian,
        args: Self::Args<'_>,
    ) -> binrw::BinResult<()> {
        self.music_trans_node_params.write_args(writer, args)?;
        self.num_play_list_items.write_le(writer)?;
        self.play_list_items.write_args(writer, args)?;
        Ok(())
    }
}

#[binrw]
#[brw(little)]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct AkMusicRanSeqPlaylistItem {
    segment_id: u32,
    play_list_item_id: i32,
    num_children: u32,
    rs_type: u32,
    r#loop: i16,
    loop_min: i16,
    loop_max: i16,
    weight: u32,
    avoid_repeat_count: u16,
    is_using_weight: u8,
    is_shuffle: u8,
    #[br(count = num_children)]
    play_list: Vec<AkMusicRanSeqPlaylistItem>,
}

#[binrw]
#[brw(little)]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct MusicTransNodeParams {
    music_node_params: MusicNodeParams,
    num_rules: u32,
    #[br(count = num_rules)]
    rules: Vec<AkMusicTransitionRule>,
}

#[binrw]
#[brw(little)]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct AkMusicTransitionRule {
    num_src: u32,
    src_id: u32,
    num_dst: u32,
    dst_id: u32,
    src_rule: AkMusicTransSrcRule,
    dst_rule: AkMusicTransDstRule,
    alloc_trans_object_flag: u8,
}

#[binrw]
#[brw(little)]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct AkMusicTransSrcRule {
    transition_time: i32,
    fade_curve: u32,
    fade_offset: i32,
    sync_type: u32,
    cue_filter_hash: u32,
    play_post_exit: u8,
}

#[binrw]
#[brw(little)]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct AkMusicTransDstRule {
    transition_time: i32,
    fade_curve: u32,
    fade_offset: i32,
    cue_filter_hash: u32,
    jump_to_id: u32,
    jump_to_type: u16,
    entry_type: u16,
    play_pre_entry: u8,
    dest_match_source_cue_name: u8,
}
