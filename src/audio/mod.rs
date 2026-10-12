use bevy::prelude::*;
use bevy::audio::AudioSource;
use hound::{SampleFormat, WavSpec, WavWriter};
use std::io::Cursor;
use std::sync::Arc;

use crate::world::chunk::BlockType;

const SAMPLE_RATE: u32 = 22050;
const STEP_VARIANTS: usize = 4;

// ============================================================
// Процедурный звук в стиле Minecraft.
//
// Ломание / установка / удар — короткие (0.10–0.25 сек) с
// резкой атакой и разбросом питча при воспроизведении.
//
// Шаги — длинные (~0.5 сек) с плавным затуханием, чтобы
// при ходьбе накладываться друг на друга и звучать непрерывно.
// ============================================================

#[derive(Resource)]
pub struct SoundHandles {
    pub place_block: Handle<AudioSource>,
    pub hit_mob: Handle<AudioSource>,
    pub break_soft: Handle<AudioSource>,
    pub break_stone: Handle<AudioSource>,
    pub break_wood: Handle<AudioSource>,
    pub break_glass: Handle<AudioSource>,
    pub break_brick: Handle<AudioSource>,
    pub step_soft: Vec<Handle<AudioSource>>,
    pub step_stone: Vec<Handle<AudioSource>>,
    pub step_wood: Vec<Handle<AudioSource>>,
    pub step_sand: Vec<Handle<AudioSource>>,
}

impl SoundHandles {
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

    pub fn step_sounds_for(&self, block: BlockType) -> &[Handle<AudioSource>] {
        use BlockType::*;
        match block {
            Grass | Dirt | Leaves | Snow => &self.step_soft,
            Sand                          => &self.step_sand,
            Log | Planks                  => &self.step_wood,
            Stone | Cobblestone | Brick | Obsidian | Glass => &self.step_stone,
            Air                           => &self.step_soft,
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

    let step_soft = (0..STEP_VARIANTS)
        .map(|i| assets.add(make_source(&gen_step_soft(i as u32))))
        .collect();
    let step_stone = (0..STEP_VARIANTS)
        .map(|i| assets.add(make_source(&gen_step_stone(i as u32))))
        .collect();
    let step_wood = (0..STEP_VARIANTS)
        .map(|i| assets.add(make_source(&gen_step_wood(i as u32))))
        .collect();
    let step_sand = (0..STEP_VARIANTS)
        .map(|i| assets.add(make_source(&gen_step_sand(i as u32))))
        .collect();

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
    info!("GameAudio: sounds generated (steps: {} variants, ~0.5s each)", STEP_VARIANTS);
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
            writer.write_sample(s).expect("write_sample should not fail");
        }
        writer.finalize().expect("finalize should not fail");
    }
    cursor.into_inner()
}

#[inline]
fn lcg_next(state: &mut u32) -> f32 {
    *state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    ((*state >> 16) as i16 as f32) / 32768.0
}

#[inline]
fn seed_from(base: u32, variant: u32) -> u32 {
    base.wrapping_mul(2_654_435_761).wrapping_add(variant.wrapping_mul(40_503))
}

// ============================================================
// Установка блока
// ============================================================
fn gen_place() -> Vec<i16> {
    let dur = 0.12_f32;
    let n = (SAMPLE_RATE as f32 * dur) as usize;
    let mut out = Vec::with_capacity(n);

    for i in 0..n {
        let t = i as f32 / SAMPLE_RATE as f32;
        let attack = (t / 0.002).min(1.0);
        let env = attack * (1.0 - t / dur).max(0.0).powf(2.0);

        let freq = 260.0 - 120.0 * (t / dur);
        let body = (t * freq * std::f32::consts::TAU).sin();

        let click_env = (1.0 - t / 0.006).max(0.0);
        let click = (t * 1800.0 * std::f32::consts::TAU).sin() * click_env;

        let s = body * 0.75 + click * 0.25;
        out.push((s * env * 0.7 * 32767.0) as i16);
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
// Ломание блоков
// ============================================================

fn gen_break_soft() -> Vec<i16> {
    let dur = 0.12_f32;
    let n = (SAMPLE_RATE as f32 * dur) as usize;
    let mut out = Vec::with_capacity(n);
    let mut state: u32 = 0xC0FF_EE00;
    let mut lp: f32 = 0.0;

    for i in 0..n {
        let t = i as f32 / SAMPLE_RATE as f32;
        let attack = (t / 0.003).min(1.0);
        let env = attack * (1.0 - t / dur).max(0.0).powf(2.8);

        let noise = lcg_next(&mut state);
        lp = lp * 0.82 + noise * 0.18;

        let thump_env = (1.0 - t / 0.035).max(0.0);
        let thump = (t * 90.0 * std::f32::consts::TAU).sin() * thump_env;

        let s = lp * 0.8 + thump * 0.4;
        out.push((s * env * 0.75 * 32767.0) as i16);
    }
    out
}

fn gen_break_stone() -> Vec<i16> {
    let dur = 0.16_f32;
    let n = (SAMPLE_RATE as f32 * dur) as usize;
    let mut out = Vec::with_capacity(n);
    let mut state: u32 = 0xDEAD_BEEF;
    let mut lp: f32 = 0.0;

    for i in 0..n {
        let t = i as f32 / SAMPLE_RATE as f32;
        let attack = (t / 0.001).min(1.0);
        let env = attack * (1.0 - t / dur).max(0.0).powf(1.6);

        let noise = lcg_next(&mut state);
        lp = lp * 0.45 + noise * 0.55;
        let hp = noise - lp;

        let crack_env = (1.0 - t / 0.010).max(0.0);
        let crack = (t * 2200.0 * std::f32::consts::TAU).sin() * crack_env;

        let body_env = (1.0 - t / 0.06).max(0.0);
        let body = (t * 320.0 * std::f32::consts::TAU).sin() * body_env;

        let s = hp * 0.55 + lp * 0.35 + crack * 0.35 + body * 0.25;
        out.push((s * env * 0.8 * 32767.0) as i16);
    }
    out
}

fn gen_break_wood() -> Vec<i16> {
    let dur = 0.20_f32;
    let n = (SAMPLE_RATE as f32 * dur) as usize;
    let mut out = Vec::with_capacity(n);
    let mut state: u32 = 0x1234_5678;
    let mut lp: f32 = 0.0;

    for i in 0..n {
        let t = i as f32 / SAMPLE_RATE as f32;
        let attack = (t / 0.001).min(1.0);
        let env = attack * (1.0 - t / dur).max(0.0).powf(2.0);

        let mode1 = (t * 280.0 * std::f32::consts::TAU).sin()
            * (1.0 - t / dur).max(0.0).powf(3.5);
        let mode2 = (t * 430.0 * std::f32::consts::TAU).sin()
            * (1.0 - t / 0.07).max(0.0).powf(4.5);

        let noise = lcg_next(&mut state);
        lp = lp * 0.72 + noise * 0.28;

        let snap_env = (1.0 - t / 0.005).max(0.0);
        let snap = (t * 1400.0 * std::f32::consts::TAU).sin() * snap_env;

        let s = mode1 * 0.55 + mode2 * 0.30 + lp * 0.30 + snap * 0.20;
        out.push((s * env * 0.75 * 32767.0) as i16);
    }
    out
}

fn gen_break_glass() -> Vec<i16> {
    let dur = 0.25_f32;
    let n = (SAMPLE_RATE as f32 * dur) as usize;
    let mut out = Vec::with_capacity(n);
    let mut state: u32 = 0xFACE_B00C;

    for i in 0..n {
        let t = i as f32 / SAMPLE_RATE as f32;
        let attack = (t / 0.001).min(1.0);
        let env = attack * (1.0 - t / dur).max(0.0).powf(1.2);

        let noise = lcg_next(&mut state);

        let ring1 = (t * 3200.0 * std::f32::consts::TAU).sin()
            * (1.0 - t / dur).max(0.0).powf(3.0);
        let ring2 = (t * 4800.0 * std::f32::consts::TAU).sin()
            * (1.0 - t / 0.08).max(0.0).powf(4.0);

        let s = noise * 0.35 + ring1 * 0.65 + ring2 * 0.25;
        out.push((s * env * 0.65 * 32767.0) as i16);
    }
    out
}

fn gen_break_brick() -> Vec<i16> {
    let dur = 0.15_f32;
    let n = (SAMPLE_RATE as f32 * dur) as usize;
    let mut out = Vec::with_capacity(n);
    let mut state: u32 = 0xCAFE_1111;
    let mut lp: f32 = 0.0;

    for i in 0..n {
        let t = i as f32 / SAMPLE_RATE as f32;
        let attack = (t / 0.001).min(1.0);
        let env = attack * (1.0 - t / dur).max(0.0).powf(1.8);

        let noise = lcg_next(&mut state);
        lp = lp * 0.55 + noise * 0.45;
        let hp = noise - lp;

        let tone_env = (1.0 - t / 0.05).max(0.0);
        let tone = (t * 190.0 * std::f32::consts::TAU).sin() * tone_env;

        let crack_env = (1.0 - t / 0.012).max(0.0);
        let crack = (t * 1600.0 * std::f32::consts::TAU).sin() * crack_env;

        let s = lp * 0.5 + hp * 0.4 + tone * 0.45 + crack * 0.25;
        out.push((s * env * 0.8 * 32767.0) as i16);
    }
    out
}

// ============================================================
// Шаги — длинные (~0.5 сек), с плавным затуханием.
// Атака в начале — как «касание», дальше — хвост, который
// накладывается на следующий шаг при движении.
// ============================================================

/// Мягкий шаг (трава / земля / снег / листва).
fn gen_step_soft(variant: u32) -> Vec<i16> {
    let dur = 0.50_f32;
    let n = (SAMPLE_RATE as f32 * dur) as usize;
    let mut out = Vec::with_capacity(n);
    let mut state: u32 = seed_from(0x55AA_1010, variant);
    let mut lp: f32 = 0.0;

    for i in 0..n {
        let t = i as f32 / SAMPLE_RATE as f32;

        // Атака 8 мс — «касание».
        let attack = (t / 0.008).min(1.0);
        // Хвост с плавным затуханием — 1.0 → 0.0 к 0.5 сек.
        let tail = (1.0 - t / dur).max(0.0).powf(0.7);

        let noise = lcg_next(&mut state);
        lp = lp * 0.85 + noise * 0.15;

        // Толчок 70 Гц в начале.
        let thump_env = (1.0 - t / 0.06).max(0.0);
        let thump = (t * 70.0 * std::f32::consts::TAU).sin() * thump_env;

        let s = (lp * 0.9 + thump * 0.4) * attack * tail;
        out.push((s * 0.45 * 32767.0) as i16);
    }
    out
}

/// Шаг по камню / кирпичу / стеклу / обсидиану.
fn gen_step_stone(variant: u32) -> Vec<i16> {
    let dur = 0.45_f32;
    let n = (SAMPLE_RATE as f32 * dur) as usize;
    let mut out = Vec::with_capacity(n);
    let mut state: u32 = seed_from(0xAA55_2020, variant);
    let mut lp: f32 = 0.0;

    for i in 0..n {
        let t = i as f32 / SAMPLE_RATE as f32;

        // Резкая атака — «цок».
        let attack = (t / 0.0004).min(1.0);
        // Пик 30 мс — «цок», потом хвост.
        let peak = (1.0 - t / 0.030).max(0.0).powf(2.0);
        let tail = (1.0 - t / dur).max(0.0).powf(0.6);
        let env = attack * (peak + tail * 0.35);

        let noise = lcg_next(&mut state);
        lp = lp * 0.35 + noise * 0.65;
        let hp = noise - lp;

        // Тональный «цок» 1700 Гц — только в начале.
        let click_env = (1.0 - t / 0.012).max(0.0);
        let click = (t * 1700.0 * std::f32::consts::TAU).sin() * click_env;

        let s = (hp * 0.7 + click * 0.35 + lp * 0.25) * env;
        out.push((s * 0.42 * 32767.0) as i16);
    }
    out
}

/// Шаг по дереву (доски / бревно).
fn gen_step_wood(variant: u32) -> Vec<i16> {
    let dur = 0.48_f32;
    let n = (SAMPLE_RATE as f32 * dur) as usize;
    let mut out = Vec::with_capacity(n);
    let mut state: u32 = seed_from(0x33CC_3030, variant);
    let mut lp: f32 = 0.0;

    let f1 = 250.0 + variant as f32 * 12.0;
    let f2 = f1 * 1.55;

    for i in 0..n {
        let t = i as f32 / SAMPLE_RATE as f32;

        let attack = (t / 0.001).min(1.0);
        // Моды затухают за 0.15 сек.
        let mode_env1 = (1.0 - t / 0.20).max(0.0).powf(2.5);
        let mode_env2 = (1.0 - t / 0.10).max(0.0).powf(3.5);
        // Длинный хвост — слабый шум до конца.
        let tail = (1.0 - t / dur).max(0.0).powf(0.6);

        let mode1 = (t * f1 * std::f32::consts::TAU).sin() * mode_env1;
        let mode2 = (t * f2 * std::f32::consts::TAU).sin() * mode_env2;

        let noise = lcg_next(&mut state);
        lp = lp * 0.78 + noise * 0.22;

        let s = (mode1 * 0.55 + mode2 * 0.30 + lp * 0.35 * tail) * attack;
        out.push((s * 0.45 * 32767.0) as i16);
    }
    out
}

/// Шаг по песку — длинное «шшшш».
fn gen_step_sand(variant: u32) -> Vec<i16> {
    let dur = 0.50_f32;
    let n = (SAMPLE_RATE as f32 * dur) as usize;
    let mut out = Vec::with_capacity(n);
    let mut state: u32 = seed_from(0x77EE_4040, variant);
    let mut lp: f32 = 0.0;

    for i in 0..n {
        let t = i as f32 / SAMPLE_RATE as f32;

        // Мягкая атака 25 мс — «осыпание».
        let attack = (t / 0.025).min(1.0);
        let tail = (1.0 - t / dur).max(0.0).powf(0.5);

        let noise = lcg_next(&mut state);
        lp = lp * 0.25 + noise * 0.75;

        // Амплитудная модуляция 28 Гц — «шелест».
        let am = 1.0 + 0.22 * (t * 28.0 * std::f32::consts::TAU).sin();

        let s = lp * am * attack * tail;
        out.push((s * 0.40 * 32767.0) as i16);
    }
    out
}