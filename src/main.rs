use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::f32::consts::PI;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let host = cpal::default_host();
    
    // Выбираем строго PipeWire, раз он у тебя есть
    let device = host.output_devices()?
        .find(|d| d.name().unwrap_or_default().to_lowercase().contains("pipewire"))
        .expect("PipeWire не найден");

    let config_supported = device.default_output_config()?;
    let mut config: cpal::StreamConfig = config_supported.clone().into();
    
    // Увеличим буфер, чтобы WSL успевал переваривать звук
    config.buffer_size = cpal::BufferSize::Fixed(1024);

    let sample_rate_ = config.sample_rate as f32;
    let sample_rate = sample_rate_ / 3 as f32;
    let channels = config.channels as usize;
    let mut sample_clock = 0f32;

    println!("Запуск на: {}. Частота: {} Гц", device.name()?, sample_rate);

    let stream = device.build_output_stream(
        &config,
        move |data: &mut [f32], _| {
            for frame in data.chunks_mut(channels) {
                sample_clock = (sample_clock + 1.0) % sample_rate;
                let value = (sample_clock * 440.0 * 2.0 * PI / sample_rate).sin();
                for sample in frame.iter_mut() {
                    *sample = value * 0.3; // 30% громкости
                }
            }
        },
        |err| eprintln!("Ошибка: {}", err),
        None
    )?;

    stream.play()?;
    
    println!(">>> Генерирую звук. Если тишина — не закрывай программу и читай ниже!");
    std::io::stdin().read_line(&mut String::new())?;

    Ok(())
}