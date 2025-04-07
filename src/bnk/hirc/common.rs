use crate::rwext::ReadVecExt;
use binrw::{BinRead, BinWrite, binrw};
use serde::{Deserialize, Serialize};

#[binrw]
#[brw(little)]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct NodeBaseParams {
    node_initial_fx_params: NodeInitialFxParams,
    is_override_parent_metadata: u8,
    num_fx: u8,
    override_attachment_params: u8,
    override_bus_id: u32,
    direct_parent_id: u32,
    by_bit_vector: u8,
    node_initial_params: NodeInitialParams,
    positioning_params: PositioningParams,
    aux_params: AuxParams,
    adv_settings_params: AdvSettingsParams,
    state_chunk: StateChunk,
    initial_rtpc: InitialRTPC,
}

#[binrw]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct NodeInitialFxParams {
    is_override_parent_fx: u8,
    num_fx: u8,
}

#[binrw]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct NodeInitialParams {
    ak_prop_bundle1: AkPropBundle,
    ak_prop_bundle2: AkPropBundle,
}

#[binrw]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct AkPropBundle {
    num_props: u8,
    #[br(count = num_props)]
    props: Vec<AkPropBundleElem>,
}

#[binrw]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct AkPropBundleElem {
    p_id: u8,
    p_value: u32,
}

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct PositioningParams {
    bits_positioning: u8,

    bits_3d: u8,
    is_dynamic: u8,

    e_path_mode: AkPathMode,
    transition_time: i32,
    vertices: Vec<AkPathVertex>,
    play_list_items: Vec<AkPathListItemOffset>,
    params: Vec<Ak3DAutomationParams>,
}

impl BinRead for PositioningParams {
    type Args<'a> = ();

    fn read_options<R: std::io::Read + std::io::Seek>(
        reader: &mut R,
        endian: binrw::Endian,
        args: Self::Args<'_>,
    ) -> binrw::BinResult<Self> {
        let bits_positioning = u8::read_options(reader, endian, ())?;

        let has_positioning = bits_positioning & 0x1 != 0;
        let has_3d = (bits_positioning >> 1) & 1 != 0;
        let bits_3d = if has_positioning && has_3d {
            u8::read_options(reader, endian, ())?
        } else {
            0
        };

        let e3d_position_type = (bits_positioning >> 5) & 3;
        let has_automation = e3d_position_type != 0;
        let has_dynamic = false; // const in v145
        let is_dynamic = if has_dynamic {
            u8::read_options(reader, endian, ())?
        } else {
            0
        };

        let mut this: PositioningParams = Self {
            bits_positioning,
            bits_3d,
            is_dynamic,
            ..Default::default()
        };

        if has_automation {
            this.e_path_mode = AkPathMode::read_options(reader, endian, args)?;
            this.transition_time = i32::read_options(reader, endian, ())?;
            let num_vertices = u32::read_options(reader, endian, ())? as usize;
            this.vertices = reader.read_vec_fn(num_vertices, |r| {
                AkPathVertex::read_options(r, endian, args)
            })?;
            let num_play_list_items = u32::read_options(reader, endian, ())? as usize;
            this.play_list_items = reader.read_vec_fn(num_play_list_items, |r| {
                AkPathListItemOffset::read_options(r, endian, args)
            })?;
            this.params = reader.read_vec_fn(num_play_list_items, |r| {
                Ak3DAutomationParams::read_options(r, endian, args)
            })?;
        }

        Ok(this)
    }
}

impl BinWrite for PositioningParams {
    type Args<'a> = ();

    fn write_options<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        endian: binrw::Endian,
        args: Self::Args<'_>,
    ) -> binrw::BinResult<()> {
        self.bits_positioning.write_options(writer, endian, ())?;
        if self.bits_positioning & 0x1 != 0 {
            self.bits_3d.write_options(writer, endian, ())?;
        }

        if (self.bits_positioning >> 5) & 3 != 0 {
            self.e_path_mode.write_options(writer, endian, args)?;
            self.transition_time.write_options(writer, endian, ())?;
            let num_vertices = self.vertices.len() as u32;
            num_vertices.write_options(writer, endian, ())?;
            for vertex in &self.vertices {
                vertex.write_options(writer, endian, args)?;
            }
            let num_play_list_items = self.play_list_items.len() as u32;
            num_play_list_items.write_options(writer, endian, ())?;
            for play_list_item in &self.play_list_items {
                play_list_item.write_options(writer, endian, args)?;
            }
            for param in &self.params {
                param.write_options(writer, endian, args)?;
            }
        }

        Ok(())
    }
}

#[repr(u8)]
#[binrw]
#[brw(repr(u8))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum AkPathMode {
    #[default]
    StepSequence = 0x0,
    StepRandom = 0x1,
    ContinuousSequence = 0x2,
    ContinuousRandom = 0x3,
    StepSequencePickNewPath = 0x4, // from tests, not in enum (~v134)
    StepRandomPickNewPath = 0x5,   // same
}

#[binrw]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct AkPathVertex {
    vertex_x: f32,
    vertex_y: f32,
    vertex_z: f32,
    duration: i32,
}

#[binrw]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct AkPathListItemOffset {
    vertices_offset: u32,
    num_vertices: u32,
}

#[binrw]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Ak3DAutomationParams {
    x_range: f32,
    y_range: f32,
    z_range: f32,
}

#[binrw]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct AuxParams {
    by_bit_vector: u8,
    #[brw(if(by_bit_vector & (1 << 3) != 0))]
    aux_ids: [u32; 4],
    reflections_aux_bus: u32,
}

#[binrw]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct AdvSettingsParams {
    by_bit_vector: u8,
    virtual_queue_behavior: u8,
    max_num_instance: u16,
    below_threshold_behavior: u8,
    by_bit_vector2: u8,
}

#[binrw]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct AkStatePropertyInfo {
    property_id: u8,
    accum_type: u8,
    in_db: u8,
}

#[binrw]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct AkState {
    state_id: u32,
    state_instance_id: u32,
}

#[binrw]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct AkStateGroupChunk {
    state_group_id: u32,
    state_sync_type: u8,
    num_states: u8,
    #[br(count = num_states)]
    states: Vec<AkState>,
}

#[binrw]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct StateChunk {
    num_state_props: u8,
    #[br(count = num_state_props)]
    state_props: Vec<AkStatePropertyInfo>,
    num_state_groups: u8,
    #[br(count = num_state_groups)]
    state_groups: Vec<AkStateGroupChunk>,
}

#[binrw]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct InitialRTPC {
    num_curves: u16,
    #[br(count = num_curves)]
    curves: Vec<InitialRTPCCurve>,
}

#[binrw]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct InitialRTPCCurve {
    rtpc_id: u32,
    rtpc_type: u8,
    rtpc_accum: u8,
    param_id: u8,
    rtpc_curve_id: u32,
    e_scaling: u8,
    size: u16,
    #[br(count = size)]
    rtpc_mgr: Vec<AkRTPCGraphPoint>,
}

#[binrw]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct AkRTPCGraphPoint {
    from: f32,
    to: f32,
    interp: u32,
}
