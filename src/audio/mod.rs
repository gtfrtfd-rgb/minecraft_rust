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
const STEP_VARIANTS: usize = 4;

#[derive(Resource)]
pub struct SoundHandles {
    pub hit_mob: Handle<AudioSource>,

    pub break_soft: Handle<AudioSource>,
    pub break_stone: Handle<AudioSource>,
    pub break_wood: Handle<AudioSource>,
    pub break_glass: Handle<AudioSource>,
    pub break_brick: Handle<AudioSource>,

    pub place_soft: Handle<AudioSource>,
    pub place_stone: Handle<AudioSource>,
    pub place_wood: Handle<AudioSource>,
    pub place_glass: Handle<AudioSource>,
    pub place_brick: Handle<AudioSource>,

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

    pub fn place_sound_for(&self, block: BlockType) -> Handle<AudioSource> {
        use BlockType::*;
        match block {
            Grass | Dirt | Sand | Snow | Leaves => self.place_soft.clone(),
            Stone | Cobblestone | Obsidian     => self.place_stone.clone(),
            Log | Planks                       => self.place_wood.clone(),
            Glass                              => self.place_glass.clone(),
            Brick                              => self.place_brick.clone(),
            Air                                => self.place_soft.clone(),
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
    let hit_mob = assets.add(make_source(&gen_hit()));

    let break_soft  = assets.add(make_source(&gen_break_soft()));
    let break_stone = assets.add(make_source(&gen_break_stone()));
    let break_wood  = assets.add(make_source(&gen_break_wood()));
    let break_glass = assets.add(make_source(&gen_break_glass()));
    let break_brick = assets.add(make_source(&gen_break_brick()));

    let place_soft  = assets.add(make_source(&gen_place_soft()));
    let place_stone = assets.add(make_source(&gen_place_stone()));
    let place_wood  = assets.add(make_source(&gen_place_wood()));
    let place_glass = assets.add(make_source(&gen_place_glass()));
    let place_brick = assets.add(make_source(&gen_place_brick()));

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
        hit_mob,
        break_soft, break_stone, break_wood, break_glass, break_brick,
        place_soft, place_stone, place_wood, place_glass, place_brick,
        step_soft, step_stone, step_wood, step_sand,
    });
    info!("GameAudio: procedural sounds generated in memory (steps: {} variants)", STEP_VARIANTS);
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
// Удар по мобу — «шлёпок»: шум + тон 180 Гц.
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
// ЛОМАНИЕ БЛОКОВ — 5 звуков по материалам
// ============================================================

/// Мягкие блоки: земля, трава, песок, снег, листва.
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

/// Камень: резкий «крак» с ВЧ-«цоком».
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

/// Дерево: полый «ток» с двумя модами.
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

/// Стекло: высокий звонкий «дзинь».
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

/// Кирпич: сухой «клац».
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
// УСТАНОВКА БЛОКА — 5 звуков по материалам
//
// Общая идея: короткий «тук» со спадом, но у каждого материала
// свой тембр и высота тона.
// ============================================================

/// Мягкий глухой «тук» для земли/травы/песка/снега.
/// Низкий, короткий, почти без ВЧ.
fn gen_place_soft() -> Vec<i16> {
    let dur = 0.13_f32;
    let n = (SAMPLE_RATE as f32 * dur) as usize;
    let mut out = Vec::with_capacity(n);
    let mut state: u32 = 0x5EED_0001;
    let mut lp: f32 = 0.0;

    for i in 0..n {
        let t = i as f32 / SAMPLE_RATE as f32;
        let attack = (t / 0.004).min(1.0);
        let env = attack * (1.0 - t / dur).max(0.0).powf(2.2);

        // Основной низкий тон — «подушка» под звук.
        let tone_env = (1.0 - t / 0.07).max(0.0);
        let tone = (t * 150.0 * std::f32::consts::TAU).sin() * tone_env;

        // Глухой шум.
        let noise = lcg_next(&mut state);
        lp = lp * 0.85 + noise * 0.15;

        let s = tone * 0.7 + lp * 0.5;
        out.push((s * env * 0.7 * 32767.0) as i16);
    }
    out
}

/// Звонкий «клац» для камня/булыжника/обсидиана.
fn gen_place_stone() -> Vec<i16> {
    let dur = 0.12_f32;
    let n = (SAMPLE_RATE as f32 * dur) as usize;
    let mut out = Vec::with_capacity(n);
    let mut state: u32 = 0x5EED_0002;
    let mut lp: f32 = 0.0;

    for i in 0..n {
        let t = i as f32 / SAMPLE_RATE as f32;
        let attack = (t / 0.0006).min(1.0);
        let env = attack * (1.0 - t / dur).max(0.0).powf(2.0);

        // Средний тон 420 Гц — характер камня.
        let tone_env = (1.0 - t / 0.05).max(0.0);
        let tone = (t * 420.0 * std::f32::consts::TAU).sin() * tone_env;

        // ВЧ-«клик» на атаке.
        let click_env = (1.0 - t / 0.008).max(0.0);
        let click = (t * 1800.0 * std::f32::consts::TAU).sin() * click_env;

        // Полосовой шум.
        let noise = lcg_next(&mut state);
        lp = lp * 0.4 + noise * 0.6;
        let hp = noise - lp;

        let s = tone * 0.55 + click * 0.35 + hp * 0.35 + lp * 0.15;
        out.push((s * env * 0.7 * 32767.0) as i16);
    }
    out
}

/// Полый «ток» для дерева — две моды, как у break_wood, но короче.
fn gen_place_wood() -> Vec<i16> {
    let dur = 0.14_f32;
    let n = (SAMPLE_RATE as f32 * dur) as usize;
    let mut out = Vec::with_capacity(n);
    let mut state: u32 = 0x5EED_0003;
    let mut lp: f32 = 0.0;

    for i in 0..n {
        let t = i as f32 / SAMPLE_RATE as f32;
        let attack = (t / 0.001).min(1.0);
        let env = attack * (1.0 - t / dur).max(0.0).powf(1.8);

        let mode1 = (t * 320.0 * std::f32::consts::TAU).sin()
            * (1.0 - t / 0.10).max(0.0).powf(3.0);
        let mode2 = (t * 500.0 * std::f32::consts::TAU).sin()
            * (1.0 - t / 0.05).max(0.0).powf(4.0);

        // Немного глухого шума для «деревянности».
        let noise = lcg_next(&mut state);
        lp = lp * 0.75 + noise * 0.25;

        let s = mode1 * 0.6 + mode2 * 0.3 + lp * 0.3;
        out.push((s * env * 0.7 * 32767.0) as i16);
    }
    out
}

/// Высокий «дзынь» для стекла.
fn gen_place_glass() -> Vec<i16> {
    let dur = 0.16_f32;
    let n = (SAMPLE_RATE as f32 * dur) as usize;
    let mut out = Vec::with_capacity(n);
    let mut state: u32 = 0x5EED_0004;

    for i in 0..n {
        let t = i as f32 / SAMPLE_RATE as f32;
        let attack = (t / 0.001).min(1.0);
        let env = attack * (1.0 - t / dur).max(0.0).powf(1.4);

        // Два высоких резонанса.
        let ring1 = (t * 2000.0 * std::f32::consts::TAU).sin()
            * (1.0 - t / dur).max(0.0).powf(2.8);
        let ring2 = (t * 3300.0 * std::f32::consts::TAU).sin()
            * (1.0 - t / 0.06).max(0.0).powf(3.5);

        let noise = lcg_next(&mut state);

        let s = ring1 * 0.65 + ring2 * 0.3 + noise * 0.25;
        out.push((s * env * 0.55 * 32767.0) as i16);
    }
    out
}

/// Сухой «клац» для кирпича.
fn gen_place_brick() -> Vec<i16> {
    let dur = 0.13_f32;
    let n = (SAMPLE_RATE as f32 * dur) as usize;
    let mut out = Vec::with_capacity(n);
    let mut state: u32 = 0x5EED_0005;
    let mut lp: f32 = 0.0;

    for i in 0..n {
        let t = i as f32 / SAMPLE_RATE as f32;
        let attack = (t / 0.001).min(1.0);
        let env = attack * (1.0 - t / dur).max(0.0).powf(1.7);

        // Средний тон.
        let tone_env = (1.0 - t / 0.05).max(0.0);
        let tone = (t * 260.0 * std::f32::consts::TAU).sin() * tone_env;

        // Сухой шумовой «треск» в начале.
        let crack_env = (1.0 - t / 0.010).max(0.0);
        let crack = (t * 1300.0 * std::f32::consts::TAU).sin() * crack_env;

        let noise = lcg_next(&mut state);
        lp = lp * 0.55 + noise * 0.45;
        let hp = noise - lp;

        let s = tone * 0.5 + crack * 0.35 + hp * 0.3 + lp * 0.2;
        out.push((s * env * 0.7 * 32767.0) as i16);
    }
    out
}

// ============================================================
// ШАГИ — 4 материала
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
        let attack = (t / 0.008).min(1.0);
        let tail = (1.0 - t / dur).max(0.0).powf(0.7);

        let noise = lcg_next(&mut state);
        lp = lp * 0.85 + noise * 0.15;

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
        let attack = (t / 0.0004).min(1.0);
        let peak = (1.0 - t / 0.030).max(0.0).powf(2.0);
        let tail = (1.0 - t / dur).max(0.0).powf(0.6);
        let env = attack * (peak + tail * 0.35);

        let noise = lcg_next(&mut state);
        lp = lp * 0.35 + noise * 0.65;
        let hp = noise - lp;

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
        let mode_env1 = (1.0 - t / 0.20).max(0.0).powf(2.5);
        let mode_env2 = (1.0 - t / 0.10).max(0.0).powf(3.5);
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
        let attack = (t / 0.025).min(1.0);
        let tail = (1.0 - t / dur).max(0.0).powf(0.5);

        let noise = lcg_next(&mut state);
        lp = lp * 0.25 + noise * 0.75;

        let am = 1.0 + 0.22 * (t * 28.0 * std::f32::consts::TAU).sin();

        let s = lp * am * attack * tail;
        out.push((s * 0.40 * 32767.0) as i16);
    }
    out
}