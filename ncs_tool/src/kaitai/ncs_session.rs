// This is a generated file! Please edit source .ksy file and use kaitai-struct-compiler to rebuild

#![allow(unused_imports)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(irrefutable_let_patterns)]
#![allow(unused_comparisons)]

extern crate kaitai;
use kaitai::*;
use std::convert::{TryFrom, TryInto};
use std::cell::{Ref, Cell, RefCell};
use std::rc::{Rc, Weak};

/**
 * Circuit Tracks session file. The device/web validator copies the whole file
 * into RAM and runs 31 validators over it in file order, so validator read-offset
 * equals file offset. This spec encodes the sections resolved so far.
 */

#[derive(Default, Debug, Clone)]
pub struct NcsSession {
    pub _root: SharedType<NcsSession>,
    pub _parent: SharedType<NcsSession>,
    pub _self: SharedType<Self>,
    pre_synth: RefCell<Vec<u8>>,
    _io: RefCell<BytesReader>,
    f_default_drum_choices: Cell<bool>,
    default_drum_choices: RefCell<Vec<u8>>,
    f_delay_preset: Cell<bool>,
    delay_preset: RefCell<u8>,
    f_drum_mute_states: Cell<bool>,
    drum_mute_states: RefCell<Vec<u8>>,
    f_drums: Cell<bool>,
    drums: RefCell<OptRc<NcsSession_DrumBlock>>,
    f_file_size: Cell<bool>,
    file_size: RefCell<u32>,
    f_midi: Cell<bool>,
    midi: RefCell<OptRc<NcsSession_MelodicBlock>>,
    f_midi_keyboard_octaves: Cell<bool>,
    midi_keyboard_octaves: RefCell<Vec<u8>>,
    f_midi_track_info: Cell<bool>,
    midi_track_info: RefCell<Vec<OptRc<NcsSession_TrackInfo>>>,
    f_reverb_preset: Cell<bool>,
    reverb_preset: RefCell<u8>,
    f_scale_root: Cell<bool>,
    scale_root: RefCell<u8>,
    f_scale_type: Cell<bool>,
    scale_type: RefCell<u8>,
    f_swing: Cell<bool>,
    swing: RefCell<u8>,
    f_swing_sync_rate: Cell<bool>,
    swing_sync_rate: RefCell<u8>,
    f_synth: Cell<bool>,
    synth: RefCell<OptRc<NcsSession_MelodicBlock>>,
    f_synth_track_info: Cell<bool>,
    synth_track_info: RefCell<Vec<OptRc<NcsSession_TrackInfo>>>,
    f_tempo: Cell<bool>,
    tempo: RefCell<u8>,
    f_timing_spare1: Cell<bool>,
    timing_spare1: RefCell<u32>,
    f_timing_spare2: Cell<bool>,
    timing_spare2: RefCell<u32>,
}
impl KStruct for NcsSession {
    type Root = NcsSession;
    type Parent = NcsSession;

    fn read<S: KStream>(
        self_rc: &OptRc<Self>,
        _io: &S,
        _root: SharedType<Self::Root>,
        _parent: SharedType<Self::Parent>,
    ) -> KResult<()> {
        *self_rc._io.borrow_mut() = _io.clone();
        self_rc._root.set(_root.get());
        self_rc._parent.set(_parent.get());
        self_rc._self.set(Ok(self_rc.clone()));
        let _rrc = self_rc._root.get_value().borrow().upgrade();
        let _prc = self_rc._parent.get_value().borrow().upgrade();
        let _r = _rrc.as_ref().unwrap();
        *self_rc.pre_synth.borrow_mut() = _io.read_bytes(740 as usize)?.into();
        Ok(())
    }
}
impl NcsSession {

    /**
     * per drum track, 0..64 (f_pp)
     */
    pub fn default_drum_choices(
        &self
    ) -> KResult<Ref<'_, Vec<u8>>> {
        let _io = self._io.borrow();
        let _rrc = self._root.get_value().borrow().upgrade();
        let _prc = self._parent.get_value().borrow().upgrade();
        let _r = _rrc.as_ref().unwrap();
        if self.f_default_drum_choices.get() {
            return Ok(self.default_drum_choices.borrow());
        }
        self.f_default_drum_choices.set(true);
        let _pos = _io.pos();
        _io.seek(107128 as usize)?;
        *self.default_drum_choices.borrow_mut() = Vec::new();
        let l_default_drum_choices = 4;
        for _i in 0..l_default_drum_choices {
            self.default_drum_choices.borrow_mut().push(_io.read_u1()?.into());
        }
        _io.seek(_pos)?;
        Ok(self.default_drum_choices.borrow())
    }
    pub fn delay_preset(
        &self
    ) -> KResult<Ref<'_, u8>> {
        let _io = self._io.borrow();
        let _rrc = self._root.get_value().borrow().upgrade();
        let _prc = self._parent.get_value().borrow().upgrade();
        let _r = _rrc.as_ref().unwrap();
        if self.f_delay_preset.get() {
            return Ok(self.delay_preset.borrow());
        }
        self.f_delay_preset.set(true);
        let _pos = _io.pos();
        _io.seek(158990 as usize)?;
        *self.delay_preset.borrow_mut() = _io.read_u1()?.into();
        if !(((*self.delay_preset()? as u8) >= (0 as u8))) {
            return Err(KError::ValidationFailed(ValidationFailedError { kind: ValidationKind::LessThan, src_path: "/instances/delay_preset".to_string() }));
        }
        if !(((*self.delay_preset()? as u8) <= (15 as u8))) {
            return Err(KError::ValidationFailed(ValidationFailedError { kind: ValidationKind::GreaterThan, src_path: "/instances/delay_preset".to_string() }));
        }
        _io.seek(_pos)?;
        Ok(self.delay_preset.borrow())
    }

    /**
     * per drum track, 0..1 (f_aq)
     */
    pub fn drum_mute_states(
        &self
    ) -> KResult<Ref<'_, Vec<u8>>> {
        let _io = self._io.borrow();
        let _rrc = self._root.get_value().borrow().upgrade();
        let _prc = self._parent.get_value().borrow().upgrade();
        let _r = _rrc.as_ref().unwrap();
        if self.f_drum_mute_states.get() {
            return Ok(self.drum_mute_states.borrow());
        }
        self.f_drum_mute_states.set(true);
        let _pos = _io.pos();
        _io.seek(107124 as usize)?;
        *self.drum_mute_states.borrow_mut() = Vec::new();
        let l_drum_mute_states = 4;
        for _i in 0..l_drum_mute_states {
            self.drum_mute_states.borrow_mut().push(_io.read_u1()?.into());
        }
        _io.seek(_pos)?;
        Ok(self.drum_mute_states.borrow())
    }

    /**
     * 4 tracks x 8 patterns x 32 steps; velocity plane at 0xCD74
     */
    pub fn drums(
        &self
    ) -> KResult<Ref<'_, OptRc<NcsSession_DrumBlock>>> {
        let _io = self._io.borrow();
        let _rrc = self._root.get_value().borrow().upgrade();
        let _prc = self._parent.get_value().borrow().upgrade();
        let _r = _rrc.as_ref().unwrap();
        if self.f_drums.get() {
            return Ok(self.drums.borrow());
        }
        let _pos = _io.pos();
        _io.seek(52596 as usize)?;
        let t = Self::read_into::<_, NcsSession_DrumBlock>(&*_io, Some(self._root.clone()), Some(self._self.clone()))?.into();
        *self.drums.borrow_mut() = t;
        _io.seek(_pos)?;
        Ok(self.drums.borrow())
    }

    /**
     * must equal 160780 (0x2740C); validated by f_tu
     */
    pub fn file_size(
        &self
    ) -> KResult<Ref<'_, u32>> {
        let _io = self._io.borrow();
        let _rrc = self._root.get_value().borrow().upgrade();
        let _prc = self._parent.get_value().borrow().upgrade();
        let _r = _rrc.as_ref().unwrap();
        if self.f_file_size.get() {
            return Ok(self.file_size.borrow());
        }
        self.f_file_size.set(true);
        let _pos = _io.pos();
        _io.seek(4 as usize)?;
        *self.file_size.borrow_mut() = _io.read_u4le()?.into();
        _io.seek(_pos)?;
        Ok(self.file_size.borrow())
    }

    /**
     * 2 midi tracks x 8 patterns x 32 steps; identical layout to synth
     */
    pub fn midi(
        &self
    ) -> KResult<Ref<'_, OptRc<NcsSession_MelodicBlock>>> {
        let _io = self._io.borrow();
        let _rrc = self._root.get_value().borrow().upgrade();
        let _prc = self._parent.get_value().borrow().upgrade();
        let _r = _rrc.as_ref().unwrap();
        if self.f_midi.get() {
            return Ok(self.midi.borrow());
        }
        let _pos = _io.pos();
        _io.seek(107132 as usize)?;
        let f = |t : &mut NcsSession_MelodicBlock| Ok(t.set_params((2).try_into().map_err(|_| KError::CastError)?));
        let t = Self::read_into_with_init::<_, NcsSession_MelodicBlock>(&*_io, Some(self._root.clone()), Some(self._self.clone()), &f)?.into();
        *self.midi.borrow_mut() = t;
        _io.seek(_pos)?;
        Ok(self.midi.borrow())
    }

    /**
     * per midi track; f_cn allowlist (range not asserted here)
     */
    pub fn midi_keyboard_octaves(
        &self
    ) -> KResult<Ref<'_, Vec<u8>>> {
        let _io = self._io.borrow();
        let _rrc = self._root.get_value().borrow().upgrade();
        let _prc = self._parent.get_value().borrow().upgrade();
        let _r = _rrc.as_ref().unwrap();
        if self.f_midi_keyboard_octaves.get() {
            return Ok(self.midi_keyboard_octaves.borrow());
        }
        self.f_midi_keyboard_octaves.set(true);
        let _pos = _io.pos();
        _io.seek(158992 as usize)?;
        *self.midi_keyboard_octaves.borrow_mut() = Vec::new();
        let l_midi_keyboard_octaves = 2;
        for _i in 0..l_midi_keyboard_octaves {
            self.midi_keyboard_octaves.borrow_mut().push(_io.read_u1()?.into());
        }
        _io.seek(_pos)?;
        Ok(self.midi_keyboard_octaves.borrow())
    }

    /**
     * 2 midi tracks; patch<=7, muteState<=1, sidechainPreset<=7 (f_yn)
     */
    pub fn midi_track_info(
        &self
    ) -> KResult<Ref<'_, Vec<OptRc<NcsSession_TrackInfo>>>> {
        let _io = self._io.borrow();
        let _rrc = self._root.get_value().borrow().upgrade();
        let _prc = self._parent.get_value().borrow().upgrade();
        let _r = _rrc.as_ref().unwrap();
        if self.f_midi_track_info.get() {
            return Ok(self.midi_track_info.borrow());
        }
        self.f_midi_track_info.set(true);
        let _pos = _io.pos();
        _io.seek(158972 as usize)?;
        *self.midi_track_info.borrow_mut() = Vec::new();
        let l_midi_track_info = 2;
        for _i in 0..l_midi_track_info {
            let t = Self::read_into::<_, NcsSession_TrackInfo>(&*_io, Some(self._root.clone()), Some(self._self.clone()))?.into();
            self.midi_track_info.borrow_mut().push(t);
        }
        _io.seek(_pos)?;
        Ok(self.midi_track_info.borrow())
    }
    pub fn reverb_preset(
        &self
    ) -> KResult<Ref<'_, u8>> {
        let _io = self._io.borrow();
        let _rrc = self._root.get_value().borrow().upgrade();
        let _prc = self._parent.get_value().borrow().upgrade();
        let _r = _rrc.as_ref().unwrap();
        if self.f_reverb_preset.get() {
            return Ok(self.reverb_preset.borrow());
        }
        self.f_reverb_preset.set(true);
        let _pos = _io.pos();
        _io.seek(158991 as usize)?;
        *self.reverb_preset.borrow_mut() = _io.read_u1()?.into();
        if !(((*self.reverb_preset()? as u8) >= (0 as u8))) {
            return Err(KError::ValidationFailed(ValidationFailedError { kind: ValidationKind::LessThan, src_path: "/instances/reverb_preset".to_string() }));
        }
        if !(((*self.reverb_preset()? as u8) <= (7 as u8))) {
            return Err(KError::ValidationFailed(ValidationFailedError { kind: ValidationKind::GreaterThan, src_path: "/instances/reverb_preset".to_string() }));
        }
        _io.seek(_pos)?;
        Ok(self.reverb_preset.borrow())
    }
    pub fn scale_root(
        &self
    ) -> KResult<Ref<'_, u8>> {
        let _io = self._io.borrow();
        let _rrc = self._root.get_value().borrow().upgrade();
        let _prc = self._parent.get_value().borrow().upgrade();
        let _r = _rrc.as_ref().unwrap();
        if self.f_scale_root.get() {
            return Ok(self.scale_root.borrow());
        }
        self.f_scale_root.set(true);
        let _pos = _io.pos();
        _io.seek(158988 as usize)?;
        *self.scale_root.borrow_mut() = _io.read_u1()?.into();
        if !(((*self.scale_root()? as u8) >= (0 as u8))) {
            return Err(KError::ValidationFailed(ValidationFailedError { kind: ValidationKind::LessThan, src_path: "/instances/scale_root".to_string() }));
        }
        if !(((*self.scale_root()? as u8) <= (11 as u8))) {
            return Err(KError::ValidationFailed(ValidationFailedError { kind: ValidationKind::GreaterThan, src_path: "/instances/scale_root".to_string() }));
        }
        _io.seek(_pos)?;
        Ok(self.scale_root.borrow())
    }
    pub fn scale_type(
        &self
    ) -> KResult<Ref<'_, u8>> {
        let _io = self._io.borrow();
        let _rrc = self._root.get_value().borrow().upgrade();
        let _prc = self._parent.get_value().borrow().upgrade();
        let _r = _rrc.as_ref().unwrap();
        if self.f_scale_type.get() {
            return Ok(self.scale_type.borrow());
        }
        self.f_scale_type.set(true);
        let _pos = _io.pos();
        _io.seek(158989 as usize)?;
        *self.scale_type.borrow_mut() = _io.read_u1()?.into();
        _io.seek(_pos)?;
        Ok(self.scale_type.borrow())
    }
    pub fn swing(
        &self
    ) -> KResult<Ref<'_, u8>> {
        let _io = self._io.borrow();
        let _rrc = self._root.get_value().borrow().upgrade();
        let _prc = self._parent.get_value().borrow().upgrade();
        let _r = _rrc.as_ref().unwrap();
        if self.f_swing.get() {
            return Ok(self.swing.borrow());
        }
        self.f_swing.set(true);
        let _pos = _io.pos();
        _io.seek(53 as usize)?;
        *self.swing.borrow_mut() = _io.read_u1()?.into();
        _io.seek(_pos)?;
        Ok(self.swing.borrow())
    }
    pub fn swing_sync_rate(
        &self
    ) -> KResult<Ref<'_, u8>> {
        let _io = self._io.borrow();
        let _rrc = self._root.get_value().borrow().upgrade();
        let _prc = self._parent.get_value().borrow().upgrade();
        let _r = _rrc.as_ref().unwrap();
        if self.f_swing_sync_rate.get() {
            return Ok(self.swing_sync_rate.borrow());
        }
        self.f_swing_sync_rate.set(true);
        let _pos = _io.pos();
        _io.seek(54 as usize)?;
        *self.swing_sync_rate.borrow_mut() = _io.read_u1()?.into();
        _io.seek(_pos)?;
        Ok(self.swing_sync_rate.borrow())
    }

    /**
     * 2 synth tracks x 8 patterns x 32 steps (stepInfo + 6 notes/step)
     */
    pub fn synth(
        &self
    ) -> KResult<Ref<'_, OptRc<NcsSession_MelodicBlock>>> {
        let _io = self._io.borrow();
        let _rrc = self._root.get_value().borrow().upgrade();
        let _prc = self._parent.get_value().borrow().upgrade();
        let _r = _rrc.as_ref().unwrap();
        if self.f_synth.get() {
            return Ok(self.synth.borrow());
        }
        let _pos = _io.pos();
        _io.seek(740 as usize)?;
        let f = |t : &mut NcsSession_MelodicBlock| Ok(t.set_params((2).try_into().map_err(|_| KError::CastError)?));
        let t = Self::read_into_with_init::<_, NcsSession_MelodicBlock>(&*_io, Some(self._root.clone()), Some(self._self.clone()), &f)?.into();
        *self.synth.borrow_mut() = t;
        _io.seek(_pos)?;
        Ok(self.synth.borrow())
    }

    /**
     * 2 synth tracks; patch<128, muteState<=1, sidechainPreset<=7 (f_ds)
     */
    pub fn synth_track_info(
        &self
    ) -> KResult<Ref<'_, Vec<OptRc<NcsSession_TrackInfo>>>> {
        let _io = self._io.borrow();
        let _rrc = self._root.get_value().borrow().upgrade();
        let _prc = self._parent.get_value().borrow().upgrade();
        let _r = _rrc.as_ref().unwrap();
        if self.f_synth_track_info.get() {
            return Ok(self.synth_track_info.borrow());
        }
        self.f_synth_track_info.set(true);
        let _pos = _io.pos();
        _io.seek(52580 as usize)?;
        *self.synth_track_info.borrow_mut() = Vec::new();
        let l_synth_track_info = 2;
        for _i in 0..l_synth_track_info {
            let t = Self::read_into::<_, NcsSession_TrackInfo>(&*_io, Some(self._root.clone()), Some(self._self.clone()))?.into();
            self.synth_track_info.borrow_mut().push(t);
        }
        _io.seek(_pos)?;
        Ok(self.synth_track_info.borrow())
    }
    pub fn tempo(
        &self
    ) -> KResult<Ref<'_, u8>> {
        let _io = self._io.borrow();
        let _rrc = self._root.get_value().borrow().upgrade();
        let _prc = self._parent.get_value().borrow().upgrade();
        let _r = _rrc.as_ref().unwrap();
        if self.f_tempo.get() {
            return Ok(self.tempo.borrow());
        }
        self.f_tempo.set(true);
        let _pos = _io.pos();
        _io.seek(52 as usize)?;
        *self.tempo.borrow_mut() = _io.read_u1()?.into();
        if !(((*self.tempo()? as u8) >= (40 as u8))) {
            return Err(KError::ValidationFailed(ValidationFailedError { kind: ValidationKind::LessThan, src_path: "/instances/tempo".to_string() }));
        }
        if !(*self.tempo()? <= 240) {
            return Err(KError::ValidationFailed(ValidationFailedError { kind: ValidationKind::GreaterThan, src_path: "/instances/tempo".to_string() }));
        }
        _io.seek(_pos)?;
        Ok(self.tempo.borrow())
    }
    pub fn timing_spare1(
        &self
    ) -> KResult<Ref<'_, u32>> {
        let _io = self._io.borrow();
        let _rrc = self._root.get_value().borrow().upgrade();
        let _prc = self._parent.get_value().borrow().upgrade();
        let _r = _rrc.as_ref().unwrap();
        if self.f_timing_spare1.get() {
            return Ok(self.timing_spare1.borrow());
        }
        self.f_timing_spare1.set(true);
        let _pos = _io.pos();
        _io.seek(56 as usize)?;
        *self.timing_spare1.borrow_mut() = _io.read_u4le()?.into();
        _io.seek(_pos)?;
        Ok(self.timing_spare1.borrow())
    }
    pub fn timing_spare2(
        &self
    ) -> KResult<Ref<'_, u32>> {
        let _io = self._io.borrow();
        let _rrc = self._root.get_value().borrow().upgrade();
        let _prc = self._parent.get_value().borrow().upgrade();
        let _r = _rrc.as_ref().unwrap();
        if self.f_timing_spare2.get() {
            return Ok(self.timing_spare2.borrow());
        }
        self.f_timing_spare2.set(true);
        let _pos = _io.pos();
        _io.seek(60 as usize)?;
        *self.timing_spare2.borrow_mut() = _io.read_u4le()?.into();
        _io.seek(_pos)?;
        Ok(self.timing_spare2.borrow())
    }
}

/**
 * header, feature flags, timing, scenes, scene/pattern chains (partially decoded; see instances)
 */
impl NcsSession {
    pub fn pre_synth(&self) -> Ref<'_, Vec<u8>> {
        self.pre_synth.borrow()
    }
}
impl NcsSession {
    pub fn _io(&self) -> Ref<'_, BytesReader> {
        self._io.borrow()
    }
}

/**
 * Structure-of-arrays. Each of 4 tracks (stride 0x3540) holds 8 patterns
 * (stride 0x6A8); each pattern has four 32-byte plane arrays at +0x00
 * (velocity), +0x20 (probability), +0x40 (drumChoice), +0x60 (drumRhythm).
 * Modeled per-pattern below; higher-level track indexing done in code.
 */

#[derive(Default, Debug, Clone)]
pub struct NcsSession_DrumBlock {
    pub _root: SharedType<NcsSession>,
    pub _parent: SharedType<NcsSession>,
    pub _self: SharedType<Self>,
    tracks: RefCell<Vec<OptRc<NcsSession_DrumTrack>>>,
    _io: RefCell<BytesReader>,
}
impl KStruct for NcsSession_DrumBlock {
    type Root = NcsSession;
    type Parent = NcsSession;

    fn read<S: KStream>(
        self_rc: &OptRc<Self>,
        _io: &S,
        _root: SharedType<Self::Root>,
        _parent: SharedType<Self::Parent>,
    ) -> KResult<()> {
        *self_rc._io.borrow_mut() = _io.clone();
        self_rc._root.set(_root.get());
        self_rc._parent.set(_parent.get());
        self_rc._self.set(Ok(self_rc.clone()));
        let _rrc = self_rc._root.get_value().borrow().upgrade();
        let _prc = self_rc._parent.get_value().borrow().upgrade();
        let _r = _rrc.as_ref().unwrap();
        *self_rc.tracks.borrow_mut() = Vec::new();
        let l_tracks = 4;
        for _i in 0..l_tracks {
            let t = Self::read_into::<_, NcsSession_DrumTrack>(&*_io, Some(self_rc._root.clone()), Some(self_rc._self.clone()))?.into();
            self_rc.tracks.borrow_mut().push(t);
        }
        Ok(())
    }
}
impl NcsSession_DrumBlock {
}
impl NcsSession_DrumBlock {
    pub fn tracks(&self) -> Ref<'_, Vec<OptRc<NcsSession_DrumTrack>>> {
        self.tracks.borrow()
    }
}
impl NcsSession_DrumBlock {
    pub fn _io(&self) -> Ref<'_, BytesReader> {
        self._io.borrow()
    }
}

#[derive(Default, Debug, Clone)]
pub struct NcsSession_DrumPattern {
    pub _root: SharedType<NcsSession>,
    pub _parent: SharedType<NcsSession_DrumTrack>,
    pub _self: SharedType<Self>,
    velocity: RefCell<Vec<u8>>,
    probability: RefCell<Vec<u8>>,
    drum_choice: RefCell<Vec<u8>>,
    drum_rhythm: RefCell<Vec<u8>>,
    playback_start: RefCell<u8>,
    playback_end: RefCell<u8>,
    sync_rate: RefCell<u8>,
    playback_direction: RefCell<u8>,
    unknown_132_167: RefCell<Vec<u8>>,
    automation: RefCell<Vec<Vec<u8>>>,
    _io: RefCell<BytesReader>,
}
impl KStruct for NcsSession_DrumPattern {
    type Root = NcsSession;
    type Parent = NcsSession_DrumTrack;

    fn read<S: KStream>(
        self_rc: &OptRc<Self>,
        _io: &S,
        _root: SharedType<Self::Root>,
        _parent: SharedType<Self::Parent>,
    ) -> KResult<()> {
        *self_rc._io.borrow_mut() = _io.clone();
        self_rc._root.set(_root.get());
        self_rc._parent.set(_parent.get());
        self_rc._self.set(Ok(self_rc.clone()));
        let _rrc = self_rc._root.get_value().borrow().upgrade();
        let _prc = self_rc._parent.get_value().borrow().upgrade();
        let _r = _rrc.as_ref().unwrap();
        *self_rc.velocity.borrow_mut() = _io.read_bytes(32 as usize)?.into();
        *self_rc.probability.borrow_mut() = _io.read_bytes(32 as usize)?.into();
        *self_rc.drum_choice.borrow_mut() = _io.read_bytes(32 as usize)?.into();
        *self_rc.drum_rhythm.borrow_mut() = _io.read_bytes(32 as usize)?.into();
        *self_rc.playback_start.borrow_mut() = _io.read_u1()?.into();
        if !(((*self_rc.playback_start() as u8) >= (0 as u8))) {
            return Err(KError::ValidationFailed(ValidationFailedError { kind: ValidationKind::LessThan, src_path: "/types/drum_pattern/seq/4".to_string() }));
        }
        if !(((*self_rc.playback_start() as u8) <= (31 as u8))) {
            return Err(KError::ValidationFailed(ValidationFailedError { kind: ValidationKind::GreaterThan, src_path: "/types/drum_pattern/seq/4".to_string() }));
        }
        *self_rc.playback_end.borrow_mut() = _io.read_u1()?.into();
        if !(((*self_rc.playback_end() as u8) >= (0 as u8))) {
            return Err(KError::ValidationFailed(ValidationFailedError { kind: ValidationKind::LessThan, src_path: "/types/drum_pattern/seq/5".to_string() }));
        }
        if !(((*self_rc.playback_end() as u8) <= (31 as u8))) {
            return Err(KError::ValidationFailed(ValidationFailedError { kind: ValidationKind::GreaterThan, src_path: "/types/drum_pattern/seq/5".to_string() }));
        }
        *self_rc.sync_rate.borrow_mut() = _io.read_u1()?.into();
        if !(((*self_rc.sync_rate() as u8) >= (0 as u8))) {
            return Err(KError::ValidationFailed(ValidationFailedError { kind: ValidationKind::LessThan, src_path: "/types/drum_pattern/seq/6".to_string() }));
        }
        if !(((*self_rc.sync_rate() as u8) <= (7 as u8))) {
            return Err(KError::ValidationFailed(ValidationFailedError { kind: ValidationKind::GreaterThan, src_path: "/types/drum_pattern/seq/6".to_string() }));
        }
        *self_rc.playback_direction.borrow_mut() = _io.read_u1()?.into();
        if !(((*self_rc.playback_direction() as u8) >= (0 as u8))) {
            return Err(KError::ValidationFailed(ValidationFailedError { kind: ValidationKind::LessThan, src_path: "/types/drum_pattern/seq/7".to_string() }));
        }
        if !(((*self_rc.playback_direction() as u8) <= (3 as u8))) {
            return Err(KError::ValidationFailed(ValidationFailedError { kind: ValidationKind::GreaterThan, src_path: "/types/drum_pattern/seq/7".to_string() }));
        }
        *self_rc.unknown_132_167.borrow_mut() = _io.read_bytes(36 as usize)?.into();
        *self_rc.automation.borrow_mut() = Vec::new();
        let l_automation = 8;
        for _i in 0..l_automation {
            self_rc.automation.borrow_mut().push(_io.read_bytes(192 as usize)?.into());
        }
        Ok(())
    }
}
impl NcsSession_DrumPattern {
}
impl NcsSession_DrumPattern {
    pub fn velocity(&self) -> Ref<'_, Vec<u8>> {
        self.velocity.borrow()
    }
}
impl NcsSession_DrumPattern {
    pub fn probability(&self) -> Ref<'_, Vec<u8>> {
        self.probability.borrow()
    }
}
impl NcsSession_DrumPattern {
    pub fn drum_choice(&self) -> Ref<'_, Vec<u8>> {
        self.drum_choice.borrow()
    }
}
impl NcsSession_DrumPattern {
    pub fn drum_rhythm(&self) -> Ref<'_, Vec<u8>> {
        self.drum_rhythm.borrow()
    }
}
impl NcsSession_DrumPattern {
    pub fn playback_start(&self) -> Ref<'_, u8> {
        self.playback_start.borrow()
    }
}
impl NcsSession_DrumPattern {
    pub fn playback_end(&self) -> Ref<'_, u8> {
        self.playback_end.borrow()
    }
}
impl NcsSession_DrumPattern {
    pub fn sync_rate(&self) -> Ref<'_, u8> {
        self.sync_rate.borrow()
    }
}
impl NcsSession_DrumPattern {
    pub fn playback_direction(&self) -> Ref<'_, u8> {
        self.playback_direction.borrow()
    }
}

/**
 * no validator — carried raw (pending decode)
 */
impl NcsSession_DrumPattern {
    pub fn unknown_132_167(&self) -> Ref<'_, Vec<u8>> {
        self.unknown_132_167.borrow()
    }
}

/**
 * 8 lanes x 192 bytes; values checked against an allowlist by f_lq
 */
impl NcsSession_DrumPattern {
    pub fn automation(&self) -> Ref<'_, Vec<Vec<u8>>> {
        self.automation.borrow()
    }
}
impl NcsSession_DrumPattern {
    pub fn _io(&self) -> Ref<'_, BytesReader> {
        self._io.borrow()
    }
}

#[derive(Default, Debug, Clone)]
pub struct NcsSession_DrumTrack {
    pub _root: SharedType<NcsSession>,
    pub _parent: SharedType<NcsSession_DrumBlock>,
    pub _self: SharedType<Self>,
    patterns: RefCell<Vec<OptRc<NcsSession_DrumPattern>>>,
    track_pad: RefCell<Vec<u8>>,
    _io: RefCell<BytesReader>,
}
impl KStruct for NcsSession_DrumTrack {
    type Root = NcsSession;
    type Parent = NcsSession_DrumBlock;

    fn read<S: KStream>(
        self_rc: &OptRc<Self>,
        _io: &S,
        _root: SharedType<Self::Root>,
        _parent: SharedType<Self::Parent>,
    ) -> KResult<()> {
        *self_rc._io.borrow_mut() = _io.clone();
        self_rc._root.set(_root.get());
        self_rc._parent.set(_parent.get());
        self_rc._self.set(Ok(self_rc.clone()));
        let _rrc = self_rc._root.get_value().borrow().upgrade();
        let _prc = self_rc._parent.get_value().borrow().upgrade();
        let _r = _rrc.as_ref().unwrap();
        *self_rc.patterns.borrow_mut() = Vec::new();
        let l_patterns = 8;
        for _i in 0..l_patterns {
            let t = Self::read_into::<_, NcsSession_DrumPattern>(&*_io, Some(self_rc._root.clone()), Some(self_rc._self.clone()))?.into();
            self_rc.patterns.borrow_mut().push(t);
        }
        *self_rc.track_pad.borrow_mut() = _io.read_bytes(((13632 as i32) - (((8 as i32) * (1704 as i32)) as i32)) as usize)?.into();
        Ok(())
    }
}
impl NcsSession_DrumTrack {
}
impl NcsSession_DrumTrack {
    pub fn patterns(&self) -> Ref<'_, Vec<OptRc<NcsSession_DrumPattern>>> {
        self.patterns.borrow()
    }
}

/**
 * remainder of track stride after 8 patterns
 */
impl NcsSession_DrumTrack {
    pub fn track_pad(&self) -> Ref<'_, Vec<u8>> {
        self.track_pad.borrow()
    }
}
impl NcsSession_DrumTrack {
    pub fn _io(&self) -> Ref<'_, BytesReader> {
        self._io.borrow()
    }
}

#[derive(Default, Debug, Clone)]
pub struct NcsSession_MelodicBlock {
    pub _root: SharedType<NcsSession>,
    pub _parent: SharedType<NcsSession>,
    pub _self: SharedType<Self>,
    track_count: RefCell<u32>,
    tracks: RefCell<Vec<OptRc<NcsSession_MelodicTrack>>>,
    _io: RefCell<BytesReader>,
}
impl KStruct for NcsSession_MelodicBlock {
    type Root = NcsSession;
    type Parent = NcsSession;

    fn read<S: KStream>(
        self_rc: &OptRc<Self>,
        _io: &S,
        _root: SharedType<Self::Root>,
        _parent: SharedType<Self::Parent>,
    ) -> KResult<()> {
        *self_rc._io.borrow_mut() = _io.clone();
        self_rc._root.set(_root.get());
        self_rc._parent.set(_parent.get());
        self_rc._self.set(Ok(self_rc.clone()));
        let _rrc = self_rc._root.get_value().borrow().upgrade();
        let _prc = self_rc._parent.get_value().borrow().upgrade();
        let _r = _rrc.as_ref().unwrap();
        *self_rc.tracks.borrow_mut() = Vec::new();
        let l_tracks = *self_rc.track_count();
        for _i in 0..l_tracks {
            let t = Self::read_into::<_, NcsSession_MelodicTrack>(&*_io, Some(self_rc._root.clone()), Some(self_rc._self.clone()))?.into();
            self_rc.tracks.borrow_mut().push(t);
        }
        Ok(())
    }
}
impl NcsSession_MelodicBlock {
    pub fn track_count(&self) -> Ref<'_, u32> {
        self.track_count.borrow()
    }
}
impl NcsSession_MelodicBlock {
    pub fn set_params(&mut self, track_count: u32) {
        *self.track_count.borrow_mut() = track_count;
    }
}
impl NcsSession_MelodicBlock {
}
impl NcsSession_MelodicBlock {
    pub fn tracks(&self) -> Ref<'_, Vec<OptRc<NcsSession_MelodicTrack>>> {
        self.tracks.borrow()
    }
}
impl NcsSession_MelodicBlock {
    pub fn _io(&self) -> Ref<'_, BytesReader> {
        self._io.borrow()
    }
}

#[derive(Default, Debug, Clone)]
pub struct NcsSession_MelodicPattern {
    pub _root: SharedType<NcsSession>,
    pub _parent: SharedType<NcsSession_MelodicTrack>,
    pub _self: SharedType<Self>,
    steps: RefCell<Vec<OptRc<NcsSession_MelodicStep>>>,
    playback_start: RefCell<u8>,
    playback_end: RefCell<u8>,
    sync_rate: RefCell<u8>,
    playback_direction: RefCell<u8>,
    unknown_900_935: RefCell<Vec<u8>>,
    automation: RefCell<Vec<Vec<u8>>>,
    _io: RefCell<BytesReader>,
}
impl KStruct for NcsSession_MelodicPattern {
    type Root = NcsSession;
    type Parent = NcsSession_MelodicTrack;

    fn read<S: KStream>(
        self_rc: &OptRc<Self>,
        _io: &S,
        _root: SharedType<Self::Root>,
        _parent: SharedType<Self::Parent>,
    ) -> KResult<()> {
        *self_rc._io.borrow_mut() = _io.clone();
        self_rc._root.set(_root.get());
        self_rc._parent.set(_parent.get());
        self_rc._self.set(Ok(self_rc.clone()));
        let _rrc = self_rc._root.get_value().borrow().upgrade();
        let _prc = self_rc._parent.get_value().borrow().upgrade();
        let _r = _rrc.as_ref().unwrap();
        *self_rc.steps.borrow_mut() = Vec::new();
        let l_steps = 32;
        for _i in 0..l_steps {
            let t = Self::read_into::<_, NcsSession_MelodicStep>(&*_io, Some(self_rc._root.clone()), Some(self_rc._self.clone()))?.into();
            self_rc.steps.borrow_mut().push(t);
        }
        *self_rc.playback_start.borrow_mut() = _io.read_u1()?.into();
        if !(((*self_rc.playback_start() as u8) >= (0 as u8))) {
            return Err(KError::ValidationFailed(ValidationFailedError { kind: ValidationKind::LessThan, src_path: "/types/melodic_pattern/seq/1".to_string() }));
        }
        if !(((*self_rc.playback_start() as u8) <= (31 as u8))) {
            return Err(KError::ValidationFailed(ValidationFailedError { kind: ValidationKind::GreaterThan, src_path: "/types/melodic_pattern/seq/1".to_string() }));
        }
        *self_rc.playback_end.borrow_mut() = _io.read_u1()?.into();
        if !(((*self_rc.playback_end() as u8) >= (0 as u8))) {
            return Err(KError::ValidationFailed(ValidationFailedError { kind: ValidationKind::LessThan, src_path: "/types/melodic_pattern/seq/2".to_string() }));
        }
        if !(((*self_rc.playback_end() as u8) <= (31 as u8))) {
            return Err(KError::ValidationFailed(ValidationFailedError { kind: ValidationKind::GreaterThan, src_path: "/types/melodic_pattern/seq/2".to_string() }));
        }
        *self_rc.sync_rate.borrow_mut() = _io.read_u1()?.into();
        if !(((*self_rc.sync_rate() as u8) >= (0 as u8))) {
            return Err(KError::ValidationFailed(ValidationFailedError { kind: ValidationKind::LessThan, src_path: "/types/melodic_pattern/seq/3".to_string() }));
        }
        if !(((*self_rc.sync_rate() as u8) <= (7 as u8))) {
            return Err(KError::ValidationFailed(ValidationFailedError { kind: ValidationKind::GreaterThan, src_path: "/types/melodic_pattern/seq/3".to_string() }));
        }
        *self_rc.playback_direction.borrow_mut() = _io.read_u1()?.into();
        if !(((*self_rc.playback_direction() as u8) >= (0 as u8))) {
            return Err(KError::ValidationFailed(ValidationFailedError { kind: ValidationKind::LessThan, src_path: "/types/melodic_pattern/seq/4".to_string() }));
        }
        if !(((*self_rc.playback_direction() as u8) <= (3 as u8))) {
            return Err(KError::ValidationFailed(ValidationFailedError { kind: ValidationKind::GreaterThan, src_path: "/types/melodic_pattern/seq/4".to_string() }));
        }
        *self_rc.unknown_900_935.borrow_mut() = _io.read_bytes(36 as usize)?.into();
        *self_rc.automation.borrow_mut() = Vec::new();
        let l_automation = 12;
        for _i in 0..l_automation {
            self_rc.automation.borrow_mut().push(_io.read_bytes(192 as usize)?.into());
        }
        Ok(())
    }
}
impl NcsSession_MelodicPattern {
}
impl NcsSession_MelodicPattern {
    pub fn steps(&self) -> Ref<'_, Vec<OptRc<NcsSession_MelodicStep>>> {
        self.steps.borrow()
    }
}
impl NcsSession_MelodicPattern {
    pub fn playback_start(&self) -> Ref<'_, u8> {
        self.playback_start.borrow()
    }
}
impl NcsSession_MelodicPattern {
    pub fn playback_end(&self) -> Ref<'_, u8> {
        self.playback_end.borrow()
    }
}
impl NcsSession_MelodicPattern {
    pub fn sync_rate(&self) -> Ref<'_, u8> {
        self.sync_rate.borrow()
    }
}
impl NcsSession_MelodicPattern {
    pub fn playback_direction(&self) -> Ref<'_, u8> {
        self.playback_direction.borrow()
    }
}

/**
 * no validator — carried raw (pending decode)
 */
impl NcsSession_MelodicPattern {
    pub fn unknown_900_935(&self) -> Ref<'_, Vec<u8>> {
        self.unknown_900_935.borrow()
    }
}

/**
 * 12 lanes x 192 bytes; values allowlist-checked by f_ms
 */
impl NcsSession_MelodicPattern {
    pub fn automation(&self) -> Ref<'_, Vec<Vec<u8>>> {
        self.automation.borrow()
    }
}
impl NcsSession_MelodicPattern {
    pub fn _io(&self) -> Ref<'_, BytesReader> {
        self._io.borrow()
    }
}

#[derive(Default, Debug, Clone)]
pub struct NcsSession_MelodicStep {
    pub _root: SharedType<NcsSession>,
    pub _parent: SharedType<NcsSession_MelodicPattern>,
    pub _self: SharedType<Self>,
    assigned_note_mask: RefCell<u8>,
    probability: RefCell<u8>,
    reserved: RefCell<Vec<u8>>,
    notes: RefCell<Vec<OptRc<NcsSession_Note>>>,
    _io: RefCell<BytesReader>,
}
impl KStruct for NcsSession_MelodicStep {
    type Root = NcsSession;
    type Parent = NcsSession_MelodicPattern;

    fn read<S: KStream>(
        self_rc: &OptRc<Self>,
        _io: &S,
        _root: SharedType<Self::Root>,
        _parent: SharedType<Self::Parent>,
    ) -> KResult<()> {
        *self_rc._io.borrow_mut() = _io.clone();
        self_rc._root.set(_root.get());
        self_rc._parent.set(_parent.get());
        self_rc._self.set(Ok(self_rc.clone()));
        let _rrc = self_rc._root.get_value().borrow().upgrade();
        let _prc = self_rc._parent.get_value().borrow().upgrade();
        let _r = _rrc.as_ref().unwrap();
        *self_rc.assigned_note_mask.borrow_mut() = _io.read_u1()?.into();
        if !(((*self_rc.assigned_note_mask() as u8) >= (0 as u8))) {
            return Err(KError::ValidationFailed(ValidationFailedError { kind: ValidationKind::LessThan, src_path: "/types/melodic_step/seq/0".to_string() }));
        }
        if !(((*self_rc.assigned_note_mask() as u8) <= (63 as u8))) {
            return Err(KError::ValidationFailed(ValidationFailedError { kind: ValidationKind::GreaterThan, src_path: "/types/melodic_step/seq/0".to_string() }));
        }
        *self_rc.probability.borrow_mut() = _io.read_u1()?.into();
        if !(((*self_rc.probability() as u8) >= (0 as u8))) {
            return Err(KError::ValidationFailed(ValidationFailedError { kind: ValidationKind::LessThan, src_path: "/types/melodic_step/seq/1".to_string() }));
        }
        if !(((*self_rc.probability() as u8) <= (7 as u8))) {
            return Err(KError::ValidationFailed(ValidationFailedError { kind: ValidationKind::GreaterThan, src_path: "/types/melodic_step/seq/1".to_string() }));
        }
        *self_rc.reserved.borrow_mut() = _io.read_bytes(2 as usize)?.into();
        *self_rc.notes.borrow_mut() = Vec::new();
        let l_notes = 6;
        for _i in 0..l_notes {
            let t = Self::read_into::<_, NcsSession_Note>(&*_io, Some(self_rc._root.clone()), Some(self_rc._self.clone()))?.into();
            self_rc.notes.borrow_mut().push(t);
        }
        Ok(())
    }
}
impl NcsSession_MelodicStep {
}

/**
 * bit N set => note slot N present
 */
impl NcsSession_MelodicStep {
    pub fn assigned_note_mask(&self) -> Ref<'_, u8> {
        self.assigned_note_mask.borrow()
    }
}
impl NcsSession_MelodicStep {
    pub fn probability(&self) -> Ref<'_, u8> {
        self.probability.borrow()
    }
}
impl NcsSession_MelodicStep {
    pub fn reserved(&self) -> Ref<'_, Vec<u8>> {
        self.reserved.borrow()
    }
}
impl NcsSession_MelodicStep {
    pub fn notes(&self) -> Ref<'_, Vec<OptRc<NcsSession_Note>>> {
        self.notes.borrow()
    }
}
impl NcsSession_MelodicStep {
    pub fn _io(&self) -> Ref<'_, BytesReader> {
        self._io.borrow()
    }
}

#[derive(Default, Debug, Clone)]
pub struct NcsSession_MelodicTrack {
    pub _root: SharedType<NcsSession>,
    pub _parent: SharedType<NcsSession_MelodicBlock>,
    pub _self: SharedType<Self>,
    patterns: RefCell<Vec<OptRc<NcsSession_MelodicPattern>>>,
    _io: RefCell<BytesReader>,
}
impl KStruct for NcsSession_MelodicTrack {
    type Root = NcsSession;
    type Parent = NcsSession_MelodicBlock;

    fn read<S: KStream>(
        self_rc: &OptRc<Self>,
        _io: &S,
        _root: SharedType<Self::Root>,
        _parent: SharedType<Self::Parent>,
    ) -> KResult<()> {
        *self_rc._io.borrow_mut() = _io.clone();
        self_rc._root.set(_root.get());
        self_rc._parent.set(_parent.get());
        self_rc._self.set(Ok(self_rc.clone()));
        let _rrc = self_rc._root.get_value().borrow().upgrade();
        let _prc = self_rc._parent.get_value().borrow().upgrade();
        let _r = _rrc.as_ref().unwrap();
        *self_rc.patterns.borrow_mut() = Vec::new();
        let l_patterns = 8;
        for _i in 0..l_patterns {
            let t = Self::read_into::<_, NcsSession_MelodicPattern>(&*_io, Some(self_rc._root.clone()), Some(self_rc._self.clone()))?.into();
            self_rc.patterns.borrow_mut().push(t);
        }
        Ok(())
    }
}
impl NcsSession_MelodicTrack {
}
impl NcsSession_MelodicTrack {
    pub fn patterns(&self) -> Ref<'_, Vec<OptRc<NcsSession_MelodicPattern>>> {
        self.patterns.borrow()
    }
}
impl NcsSession_MelodicTrack {
    pub fn _io(&self) -> Ref<'_, BytesReader> {
        self._io.borrow()
    }
}

#[derive(Default, Debug, Clone)]
pub struct NcsSession_Note {
    pub _root: SharedType<NcsSession>,
    pub _parent: SharedType<NcsSession_MelodicStep>,
    pub _self: SharedType<Self>,
    note_number: RefCell<u8>,
    gate: RefCell<u8>,
    delay: RefCell<u8>,
    velocity: RefCell<u8>,
    _io: RefCell<BytesReader>,
}
impl KStruct for NcsSession_Note {
    type Root = NcsSession;
    type Parent = NcsSession_MelodicStep;

    fn read<S: KStream>(
        self_rc: &OptRc<Self>,
        _io: &S,
        _root: SharedType<Self::Root>,
        _parent: SharedType<Self::Parent>,
    ) -> KResult<()> {
        *self_rc._io.borrow_mut() = _io.clone();
        self_rc._root.set(_root.get());
        self_rc._parent.set(_parent.get());
        self_rc._self.set(Ok(self_rc.clone()));
        let _rrc = self_rc._root.get_value().borrow().upgrade();
        let _prc = self_rc._parent.get_value().borrow().upgrade();
        let _r = _rrc.as_ref().unwrap();
        *self_rc.note_number.borrow_mut() = _io.read_u1()?.into();
        *self_rc.gate.borrow_mut() = _io.read_u1()?.into();
        *self_rc.delay.borrow_mut() = _io.read_u1()?.into();
        *self_rc.velocity.borrow_mut() = _io.read_u1()?.into();
        Ok(())
    }
}
impl NcsSession_Note {
}

/**
 * 0 = empty, else MIDI note 1..139
 */
impl NcsSession_Note {
    pub fn note_number(&self) -> Ref<'_, u8> {
        self.note_number.borrow()
    }
}
impl NcsSession_Note {
    pub fn gate(&self) -> Ref<'_, u8> {
        self.gate.borrow()
    }
}
impl NcsSession_Note {
    pub fn delay(&self) -> Ref<'_, u8> {
        self.delay.borrow()
    }
}
impl NcsSession_Note {
    pub fn velocity(&self) -> Ref<'_, u8> {
        self.velocity.borrow()
    }
}
impl NcsSession_Note {
    pub fn _io(&self) -> Ref<'_, BytesReader> {
        self._io.borrow()
    }
}

/**
 * 8-byte record; patch @+0, reserved +1, muteState @+2, sidechainPreset @+3, reserved +4..7
 */

#[derive(Default, Debug, Clone)]
pub struct NcsSession_TrackInfo {
    pub _root: SharedType<NcsSession>,
    pub _parent: SharedType<NcsSession>,
    pub _self: SharedType<Self>,
    patch: RefCell<u8>,
    reserved1: RefCell<Vec<u8>>,
    mute_state: RefCell<u8>,
    sidechain_preset: RefCell<u8>,
    reserved2: RefCell<Vec<u8>>,
    _io: RefCell<BytesReader>,
}
impl KStruct for NcsSession_TrackInfo {
    type Root = NcsSession;
    type Parent = NcsSession;

    fn read<S: KStream>(
        self_rc: &OptRc<Self>,
        _io: &S,
        _root: SharedType<Self::Root>,
        _parent: SharedType<Self::Parent>,
    ) -> KResult<()> {
        *self_rc._io.borrow_mut() = _io.clone();
        self_rc._root.set(_root.get());
        self_rc._parent.set(_parent.get());
        self_rc._self.set(Ok(self_rc.clone()));
        let _rrc = self_rc._root.get_value().borrow().upgrade();
        let _prc = self_rc._parent.get_value().borrow().upgrade();
        let _r = _rrc.as_ref().unwrap();
        *self_rc.patch.borrow_mut() = _io.read_u1()?.into();
        *self_rc.reserved1.borrow_mut() = _io.read_bytes(1 as usize)?.into();
        *self_rc.mute_state.borrow_mut() = _io.read_u1()?.into();
        if !(((*self_rc.mute_state() as u8) >= (0 as u8))) {
            return Err(KError::ValidationFailed(ValidationFailedError { kind: ValidationKind::LessThan, src_path: "/types/track_info/seq/2".to_string() }));
        }
        if !(((*self_rc.mute_state() as u8) <= (1 as u8))) {
            return Err(KError::ValidationFailed(ValidationFailedError { kind: ValidationKind::GreaterThan, src_path: "/types/track_info/seq/2".to_string() }));
        }
        *self_rc.sidechain_preset.borrow_mut() = _io.read_u1()?.into();
        if !(((*self_rc.sidechain_preset() as u8) >= (0 as u8))) {
            return Err(KError::ValidationFailed(ValidationFailedError { kind: ValidationKind::LessThan, src_path: "/types/track_info/seq/3".to_string() }));
        }
        if !(((*self_rc.sidechain_preset() as u8) <= (7 as u8))) {
            return Err(KError::ValidationFailed(ValidationFailedError { kind: ValidationKind::GreaterThan, src_path: "/types/track_info/seq/3".to_string() }));
        }
        *self_rc.reserved2.borrow_mut() = _io.read_bytes(4 as usize)?.into();
        Ok(())
    }
}
impl NcsSession_TrackInfo {
}
impl NcsSession_TrackInfo {
    pub fn patch(&self) -> Ref<'_, u8> {
        self.patch.borrow()
    }
}
impl NcsSession_TrackInfo {
    pub fn reserved1(&self) -> Ref<'_, Vec<u8>> {
        self.reserved1.borrow()
    }
}
impl NcsSession_TrackInfo {
    pub fn mute_state(&self) -> Ref<'_, u8> {
        self.mute_state.borrow()
    }
}
impl NcsSession_TrackInfo {
    pub fn sidechain_preset(&self) -> Ref<'_, u8> {
        self.sidechain_preset.borrow()
    }
}
impl NcsSession_TrackInfo {
    pub fn reserved2(&self) -> Ref<'_, Vec<u8>> {
        self.reserved2.borrow()
    }
}
impl NcsSession_TrackInfo {
    pub fn _io(&self) -> Ref<'_, BytesReader> {
        self._io.borrow()
    }
}
