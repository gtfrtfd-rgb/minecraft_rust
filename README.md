# Minecraft Rust

[![Build](https://github.com/gttrfrd-rgb/minecraft_rust/actions/workflows/build.yml/badge.svg)](https://github.com/gttrfrd-rgb/minecraft_rust/actions/workflows/build.yml)

Воксельная игра в стиле Minecraft, написанная на **Rust** с использованием движка **Bevy**.

---

## ✨ Возможности

- 🌍 **Процедурная генерация мира** — биомы, рельеф, деревья
- 🎨 **5 биомов** — равнины, лес, пустыня, снег, горы
- 🌳 **2 типа деревьев** — дуб (раскидистый) и сосна (ёлка)
- 🎯 **DDA-рейкаст** — точное определение блока под прицелом
- ⛏ **Ломание и установка блоков** — ЛКМ / ПКМ
- 🎒 **Хотбар** — 9 слотов, выбор через 1–9 или колесо мыши
- 🚶 **Физика игрока** — гравитация, прыжки, бег, приседание, полёт
- 🖱 **FPS-управление** — мышь захвачена, прицел в центре
- 💾 **Автосохранение** — RLE-сжатие + JSON, каждые 30 сек
- 🌫 **Туман расстояния** — скрывает границу мира
- 🚧 **Невидимые барьеры** — нельзя выйти за пределы мира
- ⚡ **Высокая производительность** — 150+ FPS на среднем железе

---

## 📥 Скачать

Готовые сборки публикуются на странице **[Releases](https://github.com/gttrfrd-rgb/minecraft_rust/releases/latest)**.

| Платформа | Файл | Размер |
|---|---|---|
| **Windows** (x64) | `minecraft_rust-x86_64-pc-windows-msvc.zip` | ~17 MB |
| **Linux** (x64) | `minecraft_rust-x86_64-unknown-linux-gnu.tar.gz` | ~22 MB |
| **macOS** (Apple Silicon) | `minecraft_rust-aarch64-apple-darwin.tar.gz` | ~19 MB |
| **macOS** (Intel) | `minecraft_rust-x86_64-apple-darwin.tar.gz` | ~20 MB |

**Как запустить:**
- **Windows:** распаковать zip → запустить `minecraft_rust.exe`
- **Linux/macOS:** распаковать → `chmod +x minecraft_rust` → `./minecraft_rust`

---

## 🎮 Управление

| Клавиша | Действие |
|---|---|
| **W A S D** / **← ↑ ↓ →** | Движение |
| **Мышь** | Обзор |
| **Space** | Прыжок (зажми — автопрыжок) |
| **Ctrl** | Бег |
| **Shift** | Присесть / спуск в полёте |
| **F** | Полёт (вкл/выкл) |
| **ЛКМ** | Сломать блок |
| **ПКМ** | Поставить блок из выбранного слота |
| **1 – 9** | Выбор слота в хотбаре |
| **Колесо мыши** | Переключение слотов |
| **Esc** | Выход из игры |

**Совет:** зажми **Space** и **Ctrl** одновременно — персонаж будет автоматически бежать и прыгать (bunny hop).

---

## 🛠 Установка (для разработки)

### Требования

- **Rust** — установить с [rustup.rs](https://rustup.rs/)
- **MSVC Build Tools** (только Windows) — [скачать](https://visualstudio.microsoft.com/downloads/?q=build+tools)
  - Или используй GNU-toolchain: `rustup default stable-x86_64-pc-windows-gnu`
- **Linux-зависимости** (Bevy):
  ```bash
  sudo apt install pkg-config libx11-dev libxcursor-dev libxrandr-dev \
                   libxi-dev libxinerama-dev libgl1-mesa-dev \
                   libasound2-dev libudev-dev libwayland-dev libxkbcommon-dev
  ```

### Сборка и запуск

```bash
# Клонировать репозиторий
git clone https://github.com/gttrfrd-rgb/minecraft_rust.git
cd minecraft_rust

# Собрать и запустить в release-режиме (быстрее)
cargo run --release
```

Первая сборка занимает **5–15 минут** (компиляция Bevy и зависимостей). Последующие — 30 сек – 2 мин.

### Сборка бинарника

```bash
cargo build --release
```

Готовый `.exe` / бинарник будет в `target/release/`.

---

## 📁 Структура проекта

```
minecraft_rust/
├── .github/workflows/
│   └── build.yml              # CI: автобилд для Win/Linux/macOS
├── src/
│   ├── main.rs                # Точка входа, регистрация плагинов
│   ├── core/
│   │   ├── mod.rs
│   │   └── state.rs           # Глобальные состояния, константы
│   ├── world/
│   │   ├── mod.rs
│   │   ├── chunk.rs           # Данные мира + построение меша
│   │   └── generator.rs       # Генерация рельефа и биомов
│   ├── player/
│   │   ├── mod.rs
│   │   ├── controller.rs      # Физика игрока
│   │   └── camera.rs          # Камера от 1-го лица + туман
│   ├── interaction/
│   │   ├── mod.rs
│   │   ├── raycast.rs         # DDA-рейкаст блоков
│   │   └── breaking.rs        # Ломание/установка
│   ├── mobs/                  # (заглушки) мобы
│   │   ├── mod.rs
│   │   ├── ai.rs
│   │   └── spawn.rs
│   ├── ui/
│   │   ├── mod.rs
│   │   ├── hotbar.rs          # Хотбар 9 слотов
│   │   └── hud.rs             # Прицел, координаты, FPS
│   └── save/
│       ├── mod.rs
│       └── persistence.rs     # RLE + JSON сохранения
├── Cargo.toml
└── README.md
```

Архитектура построена на **плагинах Bevy** — каждая система в отдельном файле. Легко добавлять/убирать модули в `main.rs`.

---

## 💾 Сохранения

Игра **автоматически** сохраняет мир:
- Каждые **30 секунд** во время игры
- При выходе (Esc)

Файлы сохранения лежат рядом с `.exe` в папке `saves/`:
- `saves/world.bin` — RLE-сжатые блоки (обычно ~180 KB)
- `saves/meta.json` — сид мира, позиция игрока, режим

**Чтобы начать новый мир** — удали папку `saves/`.

---

## ⚙️ Технологии

| Компонент | Технология |
|---|---|
| Язык | Rust 1.75+ |
| Движок | Bevy 0.15 |
| Графика | wgpu (Vulkan/Metal/DX12) |
| Шум | `noise` (Perlin) |
| Сохранения | `serde` + `serde_json` + RLE |
| CI/CD | GitHub Actions |

---

## 🚧 В планах

- [x] Генерация биомов и деревьев
- [x] Ломание и установка блоков
- [x] Хотбар + HUD
- [x] Сохранения мира
- [x] Автоматическая сборка под 4 платформы
- [ ] 🐷 Мобы (pig, sheep, cow, chicken)
- [ ] 🌍 Бесконечный мир (chunk streaming)
- [ ] 🔊 Звуковые эффекты
- [ ] 🎨 Меню паузы
- [ ] 🌞 Цикл дня и ночи
- [ ] 🎒 Инвентарь

---

## 🤝 Вклад

Pull request'ы приветствуются. Перед пушем убедись, что:

```bash
cargo build --release     # собирается без ошибок
cargo clippy --release    # нет warnings от clippy
```

CI автоматически проверит сборку на всех 4 платформах.

---

## 📜 Лицензия

MIT License — см. [LICENSE](LICENSE).

---

## 🙏 Благодарности

- **[Bevy](https://bevyengine.org/)** — мощный ECS-движок на Rust
- **[Notch](https://en.wikipedia.org/wiki/Markus_Persson)** — за оригинальный Minecraft
- Вдохновлено веб-версией на Three.js

---

**Made with ❤️ and 🦀 Rust**