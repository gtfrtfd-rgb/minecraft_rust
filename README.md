# Minecraft Rust

[![Build](https://github.com/gttrfrd-rgb/minecraft_rust/actions/workflows/build.yml/badge.svg)](https://github.com/gttrfrd-rgb/minecraft_rust/actions/workflows/build.yml)

Воксельная игра в стиле Minecraft, написанная на **Rust** с использованием движка **Bevy**.

---

## ✨ Возможности

- 🌍 **Процедурная генерация мира** — биомы, рельеф, деревья
- 🎨 **5 биомов** — равнины, лес, пустыня, снег, горы
- 🌳 **2 типа деревьев** — дуб (раскидистый) и сосна (ёлка)
- 🐷 **4 типа мобов** — свинья, овца, корова, курица
- 🎯 **DDA-рейкаст** — точное определение блока/моба под прицелом
- ⛏ **Ломание и установка блоков** — ЛКМ / ПКМ
- 🎒 **Хотбар** — 9 слотов, выбор через 1–9 или колесо мыши
- 🚶 **Физика игрока** — гравитация, прыжки, бег, приседание, полёт
- 🖱 **FPS-управление** — мышь захвачена, прицел в центре
- 💾 **Автосохранение** — RLE-сжатие + JSON, каждые 30 сек
- 🌫 **Туман расстояния** — скрывает границу мира
- 🎮 **Меню** — главное меню + пауза (Esc)
- ⚡ **Высокая производительность** — 150+ FPS на среднем железе

---

## 📥 Скачать

Готовые сборки: **[Releases](https://github.com/gttrfrd-rgb/minecraft_rust/releases/latest)**

| Платформа | Файл |
|---|---|
| Windows (x64) | minecraft_rust-x86_64-pc-windows-msvc.zip |
| Linux (x64) | minecraft_rust-x86_64-unknown-linux-gnu.tar.gz |
| macOS (Apple Silicon) | minecraft_rust-aarch64-apple-darwin.tar.gz |
| macOS (Intel) | minecraft_rust-x86_64-apple-darwin.tar.gz |

**Как запустить:**

- Windows: распаковать zip → запустить minecraft_rust.exe
- Linux/macOS: распаковать → chmod +x minecraft_rust → ./minecraft_rust

---

## 🎮 Управление

| Клавиша | Действие |
|---|---|
| W A S D / стрелки | Движение |
| Мышь | Обзор |
| Space | Прыжок (зажми — автопрыжок) |
| Ctrl | Бег |
| Shift | Присесть / спуск в полёте |
| F | Полёт (вкл/выкл) |
| ЛКМ | Ударить моба / сломать блок |
| ПКМ | Поставить блок |
| 1 – 9 | Выбор слота |
| Колесо мыши | Переключение слотов |
| Esc | Пауза / меню |

---

## 🛠 Установка (для разработки)

### Требования

- **Rust** — установить с rustup.rs
- **MSVC Build Tools** (Windows) — скачать с visualstudio.microsoft.com
- **Linux-зависимости** — выполнить в терминале:

      sudo apt install pkg-config libx11-dev libxcursor-dev libxrandr-dev \
                       libxi-dev libxinerama-dev libgl1-mesa-dev \
                       libasound2-dev libudev-dev libwayland-dev libxkbcommon-dev

### Сборка и запуск

    git clone https://github.com/gttrfrd-rgb/minecraft_rust.git
    cd minecraft_rust
    cargo run --release

Первая сборка — 5–15 минут. Последующие — 30 сек – 2 мин.

---

## 📁 Структура проекта

    minecraft_rust/
    ├── .github/workflows/build.yml    # CI для Win/Linux/macOS
    ├── src/
    │   ├── main.rs                    # Точка входа
    │   ├── core/state.rs              # Состояния (Menu/InGame/Paused)
    │   ├── world/
    │   │   ├── chunk.rs               # Данные мира + меш
    │   │   └── generator.rs           # Биомы + деревья
    │   ├── player/
    │   │   ├── controller.rs          # Физика игрока
    │   │   └── camera.rs              # Камера + туман
    │   ├── interaction/
    │   │   ├── raycast.rs             # DDA-рейкаст
    │   │   └── breaking.rs            # Ломание/установка/атака
    │   ├── mobs/
    │   │   ├── ai.rs                  # ИИ и анимация
    │   │   └── spawn.rs               # Спавн мобов
    │   ├── ui/
    │   │   ├── menu.rs                # Главное меню + пауза
    │   │   ├── hotbar.rs              # Хотбар
    │   │   └── hud.rs                 # Прицел, координаты, FPS
    │   └── save/persistence.rs        # RLE + JSON
    ├── Cargo.toml
    └── README.md

---

## 💾 Сохранения

Игра автоматически сохраняет мир:

- Каждые 30 секунд
- При выходе (Esc → Exit Game)

Файлы в папке saves/ рядом с .exe:

- saves/world.bin — RLE-сжатые блоки (~180 KB)
- saves/meta.json — сид, позиция игрока, режим

**Начать новый мир** — удалить папку saves/.

---

## ⚙️ Технологии

| Компонент | Технология |
|---|---|
| Язык | Rust 1.75+ |
| Движок | Bevy 0.15 |
| Графика | wgpu (Vulkan/Metal/DX12) |
| Шум | noise (Perlin) |
| Сохранения | serde + serde_json + RLE |
| CI/CD | GitHub Actions |

---

## 🚧 В планах

- [x] Генерация биомов и деревьев
- [x] Ломание и установка блоков
- [x] Хотбар + HUD
- [x] Сохранения мира
- [x] Автоматическая сборка под 4 платформы
- [x] Главное меню + пауза
- [x] Мобы с AI
- [ ] Бесконечный мир (chunk streaming)
- [ ] Звуковые эффекты
- [ ] Цикл дня и ночи
- [ ] Инвентарь
- [ ] Вода и лава

---

## 🤝 Вклад

Pull request'ы приветствуются. Перед пушем:

    cargo build --release
    cargo clippy --release

---

## 📜 Лицензия

MIT License — см. LICENSE.

---

**Made with ❤️ and 🦀 Rust**