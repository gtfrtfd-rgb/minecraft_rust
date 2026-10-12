use bevy::prelude::*;
use bevy::audio::AudioSource;
use hound::{SampleFormat, WavSpec, WavWriter};
use std::io::Cursor;
use std::sync::Arc;

use crate::world::chunk::BlockType;

// ============================================================
// Процедурный звук — генерируем WAV в память на Startup
// ============================================================

const SAMPLE_RATE: u32 = 22050;

#[derive(Resource)]
pub struct SoundHandles {
    pub place_block: Handle<AudioSource>,
    pub hit_mob: Handle<AudioSource>,
    pub break_soft: Handle<AudioSource>,
    pub break_stone: Handle<AudioSource>,
    pub break_wood: Handle<AudioSource>,
    pub break_glass: Handle<AudioSource>,
    pub break_brick: Handle<AudioSource>,
    // Шаги
    pub step_soft: Handle<AudioSource>,
    pub step_stone: Handle<AudioSource>,
    pub step_wood: Handle<AudioSource>,
    pub step_sand: Handle<AudioSource>,
}

impl SoundHandles {
    /// Звук ломания, соответствующий материалу блока.
    pub fn break_sound_for(&self, block: BlockType) -> Handle<AudioSource> {
        use BlockType::*;
        match block {
            Grass | Dirt | Sand | Snow | Leaves => self.break_soft.clone(),
            Stone | Cobblestone | Obsidian     => self.break_stone.clone(),
            Log | Planks                       => self.break_wood.clone(),
            Glass                              => self.break_glass.clone(),
            Brick                              => self.break_brick.clone(),
            Air                                => self.break_soft.clone(),
        }
    }

    /// Звук шага, соответствующий материалу блока под ногами.
    pub fn step_sound_for(&self, block: BlockType) -> Handle<AudioSource> {
        use BlockType::*;
        match block {
            Grass | Dirt | Leaves | Snow => self.step_soft.clone(),
            Sand                          => self.step_sand.clone(),
            Log | Planks                  => self.step_wood.clone(),
            Stone | Cobblestone | Brick | Obsidian | Glass => self.step_stone.clone(),
            Air                           => self.step_soft.clone(),
        }
    }
}

pub struct GameAudioPlugin;

impl Plugin for GameAudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, build_sound_handles);
        info!("GameAudioPlugin loaded.");
    }
}

fn build_sound_handles(
    mut commands: Commands,
    mut assets: ResMut<Assets<AudioSource>>,
) {
    let place_block = assets.add(make_source(&gen_place()));
    let hit_mob     = assets.add(make_source(&gen_hit()));
    let break_soft  = assets.add(make_source(&gen_break_soft()));
    let break_stone = assets.add(make_source(&gen_break_stone()));
    let break_wood  = assets.add(make_source(&gen_break_wood()));
    let break_glass = assets.add(make_source(&gen_break_glass()));
    let break_brick = assets.add(make_source(&gen_break_brick()));

    let step_soft   = assets.add(make_source(&gen_step_soft()));
    let step_stone  = assets.add(make_source(&gen_step_stone()));
    let step_wood   = assets.add(make_source(&gen_step_wood()));
    let step_sand   = assets.add(make_source(&gen_step_sand()));

    commands.insert_resource(SoundHandles {
        place_block,
        hit_mob,
        break_soft,
        break_stone,
        break_wood,
        break_glass,
        break_brick,
        step_soft,
        step_stone,
        step_wood,
        step_sand,
    });
    info!("GameAudio: 11 procedural sounds generated in memory");
}

fn make_source(samples: &[i16]) -> AudioSource {
    AudioSource {
        bytes: Arc::from(samples_to_wav(samples)),
    }
}

fn samples_to_wav(samples: &[i16]) -> Vec<u8> {
    let spec = WavSpec {
        channels: 1,
        sample_rate: SAMPLE_RATE,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };

    let mut cursor = Cursor::new(Vec::<u8>::new());
    {
        let mut writer = WavWriter::new(&mut cursor, spec)
            .expect("WavWriter::new should not fail on Vec<u8>");
        for &s in samples {
            writer
                .write_sample(s)
                .expect("write_sample should not fail on Vec<u8>");
        }
        writer
            .finalize()
            .expect("finalize should not fail on Vec<u8>");
    }
    cursor.into_inner()
}

// ------------------------------------------------------------
// Простой LCG для шума.
// ------------------------------------------------------------
#[inline]
fn lcg_next(state: &mut u32) -> f32 {
    *state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    ((*state >> 16) as i16 as f32) / 32768.0
}

// ============================================================
// Установка блока
// ============================================================
fn gen_place() -> Vec<i16> {
    let dur = 0.11_f32;
    let n = (SAMPLE_RATE as f32 * dur) as usize;
    let mut out = Vec::with_capacity(n);

    for i in 0..n {
        let t = i as f32 / SAMPLE_RATE as f32;
        let env = (1.0 - t / dur).max(0.0).powf(2.0);
        let freq = 200.0 - 80.0 * (t / dur);
        out.push(
            (env * (t * freq * std::f32::consts::TAU).sin() * 0.75 * 32767.0) as i16,
        );
    }
    out
}

// ============================================================
// Удар по мобу
// ============================================================
fn gen_hit() -> Vec<i16> {
    let dur = 0.10_f32;
    let n = (SAMPLE_RATE as f32 * dur) as usize;
    let mut out = Vec::with_capacity(n);
    let mut state: u32 = 0x00BE_EF00;

    for i in 0..n {
        let t = i as f32 / SAMPLE_RATE as f32;
        let env = (1.0 - t / dur).max(0.0).powf(1.6);
        let noise = lcg_next(&mut state);
        let tone = (t * 180.0 * std::f32::consts::TAU).sin();
        out.push(((noise * 0.6 + tone * 0.4) * env * 0.8 * 32767.0) as i16);
    }
    out
}

// ============================================================
// Ломание блоков — 5 разных звуков по материалам
// ============================================================

fn gen_break_soft() -> Vec<i16> {
    let dur = 0.13_f32;
    let n = (SAMPLE_RATE as f32 * dur) as usize;
    let mut out = Vec::with_capacity(n);
    let mut state: u32 = 0xC0FF_EE00;
    let mut lp: f32 = 0.0;

    for i in 0..n {
        let t = i as f32 / SAMPLE_RATE as f32;
        let env = (1.0 - t / dur).max(0.0).powf(2.2);
        let noise = lcg_next(&mut state);
        lp = lp * 0.80 + noise * 0.20;
        out.push((lp * env * 0.9 * 32767.0) as i16);
    }
    out
}

fn gen_break_stone() -> Vec<i16> {
    let dur = 0.15_f32;
    let n = (SAMPLE_RATE as f32 * dur) as usize;
    let mut out = Vec::with_capacity(n);
    let mut state: u32 = 0xDEAD_BEEF;
    let mut lp: f32 = 0.0;

    for i in 0..n {
        let t = i as f32 / SAMPLE_RATE as f32;
        let env = (1.0 - t / dur).max(0.0).powf(1.4);
        let noise = lcg_next(&mut state);
        lp = lp * 0.55 + noise * 0.45;
        let hp = noise - lp;
        let mixed = lp * 0.5 + hp * 0.7;
        out.push((mixed * env * 0.9 * 32767.0) as i16);
    }
    out
}

fn gen_break_wood() -> Vec<i16> {
    let dur = 0.18_f32;
    let n = (SAMPLE_RATE as f32 * dur) as usize;
    let mut out = Vec::with_capacity(n);
    let mut state: u32 = 0x1234_5678;
    let mut lp: f32 = 0.0;

    for i in 0..n {
        let t = i as f32 / SAMPLE_RATE as f32;
        let env = (1.0 - t / dur).max(0.0).powf(1.8);
        let noise = lcg_next(&mut state);
        lp = lp * 0.70 + noise * 0.30;
        let tone = (t * 220.0 * std::f32::consts::TAU).sin()
            * (1.0 - t / dur).max(0.0).powf(3.0);
        let s = lp * 0.5 + tone * 0.55;
        out.push((s * env * 0.85 * 32767.0) as i16);
    }
    out
}

fn gen_break_glass() -> Vec<i16> {
    let dur = 0.22_f32;
    let n = (SAMPLE_RATE as f32 * dur) as usize;
    let mut out = Vec::with_capacity(n);
    let mut state: u32 = 0xFACE_B00C;

    for i in 0..n {
        let t = i as f32 / SAMPLE_RATE as f32;
        let attack = (t / 0.005).min(1.0);
        let env = attack * (1.0 - t / dur).max(0.0).powf(1.2);
        let noise = lcg_next(&mut state);
        let ring = (t * 2400.0 * std::f32::consts::TAU).sin()
            * (1.0 - t / dur).max(0.0).powf(2.5);
        let s = noise * 0.4 + ring * 0.7;
        out.push((s * env * 0.7 * 32767.0) as i16);
    }
    out
}

fn gen_break_brick() -> Vec<i16> {
    let dur = 0.14_f32;
    let n = (SAMPLE_RATE as f32 * dur) as usize;
    let mut out = Vec::with_capacity(n);
    let mut state: u32 = 0xCAFE_1111;
    let mut lp: f32 = 0.0;

    for i in 0..n {
        let t = i as f32 / SAMPLE_RATE as f32;
        let env = (1.0 - t / dur).max(0.0).powf(1.7);
        let noise = lcg_next(&mut state);
        lp = lp * 0.65 + noise * 0.35;
        let tone = (t * 150.0 * std::f32::consts::TAU).sin()
            * (1.0 - t / dur).max(0.0).powf(4.0);
        let s = lp * 0.55 + tone * 0.5;
        out.push((s * env * 0.85 * 32767.0) as i16);
    }
    out
}

// ============================================================
// Шаги — короткие тихие звуки по материалам
// ============================================================

/// Мягкий шаг: по траве, земле, листве, снегу.
fn gen_step_soft() -> Vec<i16> {
    let dur = 0.08_f32;
    let n = (SAMPLE_RATE as f32 * dur) as usize;
    let mut out = Vec::with_capacity(n);
    let mut state: u32 = 0x55AA_1010;
    let mut lp: f32 = 0.0;

    for i in 0..n {
        let t = i as f32 / SAMPLE_RATE as f32;
        let env = (1.0 - t / dur).max(0.0).powf(1.5);
        let noise = lcg_next(&mut state);
        lp = lp * 0.75 + noise * 0.25;
        out.push((lp * env * 0.55 * 32767.0) as i16);
    }
    out
}

/// Жёсткий шаг: по камню, булыжнику, кирпичу, обсидиану.
fn gen_step_stone() -> Vec<i16> {
    let dur = 0.07_f32;
    let n = (SAMPLE_RATE as f32 * dur) as usize;
    let mut out = Vec::with_capacity(n);
    let mut state: u32 = 0xAA55_2020;
    let mut lp: f32 = 0.0;

    for i in 0..n {
        let t = i as f32 / SAMPLE_RATE as f32;
        let env = (1.0 - t / dur).max(0.0).powf(1.3);
        let noise = lcg_next(&mut state);
        lp = lp * 0.50 + noise * 0.50;
        let hp = noise - lp;
        let mixed = lp * 0.4 + hp * 0.7;
        out.push((mixed * env * 0.5 * 32767.0) as i16);
    }
    out
}

/// Шаг по дереву: полый "ток".
fn gen_step_wood() -> Vec<i16> {
    let dur = 0.09_f32;
    let n = (SAMPLE_RATE as f32 * dur) as usize;
    let mut out = Vec::with_capacity(n);
    let mut state: u32 = 0x33CC_3030;
    let mut lp: f32 = 0.0;

    for i in 0..n {
        let t = i as f32 / SAMPLE_RATE as f32;
        let env = (1.0 - t / dur).max(0.0).powf(1.6);
        let noise = lcg_next(&mut state);
        lp = lp * 0.65 + noise * 0.35;
        let tone = (t * 190.0 * std::f32::consts::TAU).sin()
            * (1.0 - t / dur).max(0.0).powf(3.5);
        let s = lp * 0.5 + tone * 0.55;
        out.push((s * env * 0.55 * 32767.0) as i16);
    }
    out
}

/// Шаг по песку: сильно заглушенный шорох.
fn gen_step_sand() -> Vec<i16> {
    let dur = 0.09_f32;
    let n = (SAMPLE_RATE as f32 * dur) as usize;
    let mut out = Vec::with_capacity(n);
    let mut state: u32 = 0x77EE_4040;
    let mut lp: f32 = 0.0;

    for i in 0..n {
        let t = i as f32 / SAMPLE_RATE as f32;
        let env = (1.0 - t / dur).max(0.0).powf(1.8);
        let noise = lcg_next(&mut state);
        lp = lp * 0.88 + noise * 0.12;
        out.push((lp * env * 0.6 * 32767.0) as i16);
    }
    out
}