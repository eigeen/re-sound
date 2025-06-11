mod common;
mod music_ran_sec_cntr;
mod music_segment;
mod music_track;

pub use music_ran_sec_cntr::*;
pub use music_segment::*;
pub use music_track::*;

use std::io;

use binrw::{BinRead, BinWrite, binrw};
use byteorder::{LE, ReadBytesExt, WriteBytesExt};

use super::{BnkError, Result};
use crate::rwext::ReadVecExt;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

trait EntryPayloadExt: Sized {
    fn from_reader<R>(reader: &mut R, length: u32) -> Result<Self>
    where
        R: io::Read + io::Seek;

    fn write_to<W>(&self, writer: &mut W) -> Result<()>
    where
        W: io::Write + io::Seek;

    fn fix_values(&mut self) -> Result<()> {
        Ok(())
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct HircEntry {
    pub entry_type: HircEntryType,
    pub length: u32,
    pub id: u32,
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub payload: HircEntryPayload,
}

impl HircEntry {
    pub(super) fn from_reader<R>(reader: &mut R, entry_type: HircEntryType) -> Result<Self>
    where
        R: io::Read + io::Seek,
    {
        let length = reader.read_u32::<LE>()?;
        let id = reader.read_u32::<LE>()?;
        let payload = match entry_type {
            HircEntryType::Settings => {
                HircEntryPayload::Settings(HircUnmanagedEntry::from_reader(reader, length)?)
            }
            HircEntryType::Sound => HircEntryPayload::Sound(HircSound::read_args(
                reader,
                HircSoundBinReadArgs {
                    data_length: length,
                },
            )?),
            HircEntryType::EventAction => {
                let scope = reader.read_u8()?;
                let scope = HircEventActionScope::from_repr(scope).ok_or(
                    BnkError::UnknownEventActionScope(reader.stream_position()?, scope),
                )?;
                let action_type = reader.read_u8()?;
                let action_type = HircEventActionType::from_repr(action_type)
                    .unwrap_or(HircEventActionType::Unknown(action_type));

                let game_object_id = reader.read_u32::<LE>()?;
                let _unk1 = reader.read_u8()?;
                let parameter_count = reader.read_u8()?;

                let parameter_types =
                    reader.read_vec_fn(parameter_count as usize, |r| -> Result<_> {
                        let param = r.read_u8()?;
                        let param = HircEventActionParameterType::from_repr(param)
                            .unwrap_or(HircEventActionParameterType::Unknown(param));
                        Ok(param)
                    })?;

                let parameters = reader.read_vec_u8(parameter_count as usize)?;
                let _unk2 = reader.read_u8()?;

                let data = reader.read_vec_u8(
                    (length as usize) - 13 - (size_of::<u8>() * (parameter_count as usize) * 2),
                )?;

                HircEntryPayload::EventAction(HircEventAction {
                    scope,
                    action_type,
                    game_object_id,
                    _unk1,
                    parameter_count,
                    parameter_types,
                    parameters,
                    _unk2,
                    data,
                })
            }
            HircEntryType::Event => {
                // if bank_version >= 134 => u8 else u32
                // assume u8 for now
                let action_count = reader.read_u8()?;
                let action_ids = unsafe { reader.read_vec_t_sized(action_count as usize)? };
                HircEntryPayload::Event { action_ids }
            }
            HircEntryType::RandomOrSequenceContainer => {
                HircEntryPayload::RandomOrSequenceContainer(HircUnmanagedEntry::from_reader(
                    reader, length,
                )?)
            }
            HircEntryType::SwitchContainer => {
                HircEntryPayload::SwitchContainer(HircUnmanagedEntry::from_reader(reader, length)?)
            }
            HircEntryType::ActorMixer => {
                HircEntryPayload::ActorMixer(HircUnmanagedEntry::from_reader(reader, length)?)
            }
            HircEntryType::AudioBus => {
                HircEntryPayload::AudioBus(HircUnmanagedEntry::from_reader(reader, length)?)
            }
            HircEntryType::BlendContainer => {
                HircEntryPayload::BlendContainer(HircUnmanagedEntry::from_reader(reader, length)?)
            }
            HircEntryType::MusicSegment => HircEntryPayload::MusicSegment(Box::new(
                HircMusicSegment::from_reader(reader, length)?,
            )),
            HircEntryType::MusicTrack => {
                HircEntryPayload::MusicTrack(Box::new(HircMusicTrack::from_reader(reader, length)?))
            }
            HircEntryType::MusicSwitchContainer => HircEntryPayload::MusicSwitchContainer(
                HircUnmanagedEntry::from_reader(reader, length)?,
            ),
            HircEntryType::MusicRanSeqCntr => HircEntryPayload::MusicRanSeqCntr(Box::new(
                HircMusicRanSecCntr::from_reader(reader, length)?,
            )),
            HircEntryType::Attenuation => {
                HircEntryPayload::Attenuation(HircUnmanagedEntry::from_reader(reader, length)?)
            }
            HircEntryType::DialogueEvent => {
                HircEntryPayload::DialogueEvent(HircUnmanagedEntry::from_reader(reader, length)?)
            }
            HircEntryType::MotionBus => {
                HircEntryPayload::MotionBus(HircUnmanagedEntry::from_reader(reader, length)?)
            }
            HircEntryType::MotionFx => {
                HircEntryPayload::MotionFx(HircUnmanagedEntry::from_reader(reader, length)?)
            }
            HircEntryType::Effect => {
                HircEntryPayload::Effect(HircUnmanagedEntry::from_reader(reader, length)?)
            }
            HircEntryType::AuxiliaryBus => {
                HircEntryPayload::AuxiliaryBus(HircUnmanagedEntry::from_reader(reader, length)?)
            }
            HircEntryType::Unknown(_) => {
                HircEntryPayload::Unknown(HircUnmanagedEntry::from_reader(reader, length)?)
            }
        };

        Ok(HircEntry {
            entry_type,
            length,
            id,
            payload,
        })
    }

    pub(super) fn write_to<W>(&mut self, writer: &mut W) -> Result<()>
    where
        W: io::Write + io::Seek,
    {
        writer.write_u8(self.entry_type.as_u8())?;
        // length needs to re-calculate
        writer.write_u32::<LE>(0)?;
        let start_pos = writer.stream_position()?;
        writer.write_u32::<LE>(self.id)?;

        self.payload.fix_values()?;

        match &self.payload {
            HircEntryPayload::Settings(entry) => {
                entry.write_to(writer)?;
            }
            HircEntryPayload::Sound(hirc_sound) => {
                hirc_sound.write(writer)?;
            }
            HircEntryPayload::EventAction(hirc_event_action) => {
                writer.write_u8(hirc_event_action.scope as u8)?;
                writer.write_u8(hirc_event_action.action_type.as_u8())?;
                writer.write_u32::<LE>(hirc_event_action.game_object_id)?;
                writer.write_u8(hirc_event_action._unk1)?;
                writer.write_u8(hirc_event_action.parameter_count)?;
                for parameter_type in &hirc_event_action.parameter_types {
                    writer.write_u8(parameter_type.as_u8())?;
                }
                writer.write_all(&hirc_event_action.parameters)?;
                writer.write_u8(hirc_event_action._unk2)?;

                writer.write_all(&hirc_event_action.data)?;
            }
            HircEntryPayload::Event { action_ids } => {
                writer.write_u8(action_ids.len() as u8)?;
                for action_id in action_ids {
                    writer.write_u32::<LE>(*action_id)?;
                }
            }
            HircEntryPayload::RandomOrSequenceContainer(entry) => {
                entry.write_to(writer)?;
            }
            HircEntryPayload::SwitchContainer(entry) => {
                entry.write_to(writer)?;
            }
            HircEntryPayload::ActorMixer(entry) => {
                entry.write_to(writer)?;
            }
            HircEntryPayload::AudioBus(entry) => {
                entry.write_to(writer)?;
            }
            HircEntryPayload::BlendContainer(entry) => {
                entry.write_to(writer)?;
            }
            HircEntryPayload::MusicSegment(entry) => {
                entry.write_to(writer)?;
            }
            HircEntryPayload::MusicTrack(hirc_music_track) => {
                hirc_music_track.write_to(writer)?;
            }
            HircEntryPayload::MusicSwitchContainer(entry) => {
                entry.write_to(writer)?;
            }
            HircEntryPayload::MusicRanSeqCntr(entry) => {
                entry.write_to(writer)?;
            }
            HircEntryPayload::Attenuation(entry) => {
                entry.write_to(writer)?;
            }
            HircEntryPayload::DialogueEvent(entry) => {
                entry.write_to(writer)?;
            }
            HircEntryPayload::MotionBus(entry) => {
                entry.write_to(writer)?;
            }
            HircEntryPayload::MotionFx(entry) => {
                entry.write_to(writer)?;
            }
            HircEntryPayload::Effect(entry) => {
                entry.write_to(writer)?;
            }
            HircEntryPayload::AuxiliaryBus(entry) => {
                entry.write_to(writer)?;
            }
            HircEntryPayload::Unknown(entry) => {
                entry.write_to(writer)?;
            }
        }

        let end_pos = writer.stream_position()?;
        // write length
        let length = (end_pos - start_pos) as u32;
        writer.seek(io::SeekFrom::Start(start_pos - 4))?;
        writer.write_u32::<LE>(length)?;
        writer.seek(io::SeekFrom::Start(end_pos))?;

        Ok(())
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "type", content = "content"))]
pub enum HircEntryPayload {
    Settings(HircUnmanagedEntry),
    Sound(HircSound),
    EventAction(HircEventAction),
    Event { action_ids: Vec<u32> },
    RandomOrSequenceContainer(HircUnmanagedEntry),
    SwitchContainer(HircUnmanagedEntry),
    ActorMixer(HircUnmanagedEntry),
    AudioBus(HircUnmanagedEntry),
    BlendContainer(HircUnmanagedEntry),
    MusicSegment(Box<HircMusicSegment>),
    MusicTrack(Box<HircMusicTrack>),
    MusicSwitchContainer(HircUnmanagedEntry),
    MusicRanSeqCntr(Box<HircMusicRanSecCntr>),
    Attenuation(HircUnmanagedEntry),
    DialogueEvent(HircUnmanagedEntry),
    MotionBus(HircUnmanagedEntry),
    MotionFx(HircUnmanagedEntry),
    Effect(HircUnmanagedEntry),
    AuxiliaryBus(HircUnmanagedEntry),
    Unknown(HircUnmanagedEntry),
}

impl HircEntryPayload {
    fn fix_values(&mut self) -> Result<()> {
        match self {
            HircEntryPayload::Settings(v) => v.fix_values(),
            HircEntryPayload::Sound(_) => Ok(()),
            HircEntryPayload::EventAction(_) => Ok(()),
            HircEntryPayload::Event { .. } => Ok(()),
            HircEntryPayload::RandomOrSequenceContainer(v) => v.fix_values(),
            HircEntryPayload::SwitchContainer(v) => v.fix_values(),
            HircEntryPayload::ActorMixer(v) => v.fix_values(),
            HircEntryPayload::AudioBus(v) => v.fix_values(),
            HircEntryPayload::BlendContainer(v) => v.fix_values(),
            HircEntryPayload::MusicSegment(v) => v.fix_values(),
            HircEntryPayload::MusicTrack(v) => v.fix_values(),
            HircEntryPayload::MusicSwitchContainer(v) => v.fix_values(),
            HircEntryPayload::MusicRanSeqCntr(v) => v.fix_values(),
            HircEntryPayload::Attenuation(v) => v.fix_values(),
            HircEntryPayload::DialogueEvent(v) => v.fix_values(),
            HircEntryPayload::MotionBus(v) => v.fix_values(),
            HircEntryPayload::MotionFx(v) => v.fix_values(),
            HircEntryPayload::Effect(v) => v.fix_values(),
            HircEntryPayload::AuxiliaryBus(v) => v.fix_values(),
            HircEntryPayload::Unknown(v) => v.fix_values(),
        }
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct HircUnmanagedEntry {
    pub data: Vec<u8>,
}

impl EntryPayloadExt for HircUnmanagedEntry {
    fn from_reader<R>(reader: &mut R, data_length: u32) -> Result<Self>
    where
        R: io::Read,
    {
        let mut data = vec![0; data_length as usize - 4];
        reader.read_exact(&mut data)?;
        Ok(HircUnmanagedEntry { data })
    }

    fn write_to<W>(&self, writer: &mut W) -> Result<()>
    where
        W: io::Write,
    {
        writer.write_all(&self.data)?;
        Ok(())
    }
}

#[binrw]
#[brw(little)]
#[br(import{ data_length: u32 })]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct HircSound {
    _unk1: u32,
    _unk2: u8,
    pub state: u32,
    pub audio_id: u32,
    pub source_id: u32,
    pub sound_type: HircSoundType,
    _unk3: u32,
    _unk4: u8,
    pub game_object_id: u32, // probably
    #[br(count = data_length - 31)]
    pub data: Vec<u8>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct HircEventAction {
    pub scope: HircEventActionScope,
    pub action_type: HircEventActionType,
    pub game_object_id: u32,
    _unk1: u8,
    pub parameter_count: u8,
    pub parameter_types: Vec<HircEventActionParameterType>,
    pub parameters: Vec<u8>,
    _unk2: u8,
    pub data: Vec<u8>,
}

#[binrw]
#[brw(repr(u8))]
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum HircSoundType {
    Sfx = 0,
    Voice = 1,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::FromRepr)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum HircEventActionScope {
    SwitchOrTrigger = 1,
    Global = 2,
    GameObject = 3,
    State = 4,
    All = 5,
    AllExcept = 6,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::FromRepr)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum HircEventActionType {
    Stop = 1,
    Pause = 2,
    Resume = 3,
    Play = 4,
    Trigger = 5,
    Mute = 6,
    UnMute = 7,
    SetVoicePitch = 8,
    ResetVoicePitch = 9,
    SetVpoceVolume = 10,
    ResetVoiceVolume = 11,
    SetBusVolume = 12,
    ResetBusVolume = 13,
    SetVoiceLowPassFilter = 14,
    ResetVoiceLowPassFilter = 15,
    EnableState = 16,
    DisableState = 17,
    SetState = 18,
    SetGameParameter = 19,
    ResetGameParameter = 20,
    SetSwitch = 21,
    ToggleBypass = 22,
    ResetBypassEffect = 23,
    Break = 24,
    Seek = 25,
    Unknown(u8),
}

impl HircEventActionType {
    fn as_u8(&self) -> u8 {
        match self {
            HircEventActionType::Stop => 1,
            HircEventActionType::Pause => 2,
            HircEventActionType::Resume => 3,
            HircEventActionType::Play => 4,
            HircEventActionType::Trigger => 5,
            HircEventActionType::Mute => 6,
            HircEventActionType::UnMute => 7,
            HircEventActionType::SetVoicePitch => 8,
            HircEventActionType::ResetVoicePitch => 9,
            HircEventActionType::SetVpoceVolume => 10,
            HircEventActionType::ResetVoiceVolume => 11,
            HircEventActionType::SetBusVolume => 12,
            HircEventActionType::ResetBusVolume => 13,
            HircEventActionType::SetVoiceLowPassFilter => 14,
            HircEventActionType::ResetVoiceLowPassFilter => 15,
            HircEventActionType::EnableState => 16,
            HircEventActionType::DisableState => 17,
            HircEventActionType::SetState => 18,
            HircEventActionType::SetGameParameter => 19,
            HircEventActionType::ResetGameParameter => 20,
            HircEventActionType::SetSwitch => 21,
            HircEventActionType::ToggleBypass => 22,
            HircEventActionType::ResetBypassEffect => 23,
            HircEventActionType::Break => 24,
            HircEventActionType::Seek => 25,
            HircEventActionType::Unknown(x) => *x,
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::FromRepr)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum HircEventActionParameterType {
    Delay = 0x0E,
    ParamPlay = 0x0F,
    Probability = 0x10,
    Unknown(u8),
}

impl HircEventActionParameterType {
    fn as_u8(&self) -> u8 {
        match self {
            HircEventActionParameterType::Delay => 0x0E,
            HircEventActionParameterType::ParamPlay => 0x0F,
            HircEventActionParameterType::Probability => 0x10,
            HircEventActionParameterType::Unknown(x) => *x,
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::FromRepr)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum HircEntryType {
    Settings = 1,
    Sound = 2,
    EventAction = 3,
    Event = 4,
    RandomOrSequenceContainer = 5,
    SwitchContainer = 6,
    ActorMixer = 7,
    AudioBus = 8,
    BlendContainer = 9,
    MusicSegment = 10,
    MusicTrack = 11,
    MusicSwitchContainer = 12,
    MusicRanSeqCntr = 13,
    Attenuation = 14,
    DialogueEvent = 15,
    MotionBus = 16,
    MotionFx = 17,
    Effect = 18,
    // 19 unknown
    AuxiliaryBus = 20,
    Unknown(u8),
}

impl HircEntryType {
    fn as_u8(&self) -> u8 {
        match self {
            HircEntryType::Settings => 1,
            HircEntryType::Sound => 2,
            HircEntryType::EventAction => 3,
            HircEntryType::Event => 4,
            HircEntryType::RandomOrSequenceContainer => 5,
            HircEntryType::SwitchContainer => 6,
            HircEntryType::ActorMixer => 7,
            HircEntryType::AudioBus => 8,
            HircEntryType::BlendContainer => 9,
            HircEntryType::MusicSegment => 10,
            HircEntryType::MusicTrack => 11,
            HircEntryType::MusicSwitchContainer => 12,
            HircEntryType::MusicRanSeqCntr => 13,
            HircEntryType::Attenuation => 14,
            HircEntryType::DialogueEvent => 15,
            HircEntryType::MotionBus => 16,
            HircEntryType::MotionFx => 17,
            HircEntryType::Effect => 18,
            HircEntryType::AuxiliaryBus => 20,
            HircEntryType::Unknown(x) => *x,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_action() {
        let data = &[
            0x1D, 0x00, 0x00, 0x00, 0x7F, 0x75, 0x27, 0x37, 0x03, 0x13, 0xF8, 0x2D, 0x14, 0x12,
            0x00, 0x00, 0x00, 0x04, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        let mut reader = io::Cursor::new(data);
        let hirc_entry = HircEntry::from_reader(&mut reader, HircEntryType::EventAction).unwrap();
        eprintln!("{:#?}", hirc_entry);
    }
}
