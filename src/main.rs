use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use elevator::{
    args::Args,
    controls::Controls,
    policies,
    simulation::Simulation,
    traffic::{self, Traffic},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    symbols,
    text::{Line, Span},
    widgets::{Axis, Block, Borders, Chart, Dataset, GraphType, LegendPosition, Paragraph},
};
use std::time::{Duration, Instant};

const TICK_RATE: Duration = Duration::from_millis(20);

const ELEV_COLORS: [Color; 6] = [Color::Red, Color::Green, Color::Yellow, Color::Blue, Color::Magenta, Color::Cyan];
const POLICY_COLORS: [Color; 5] = [
    Color::Indexed(99),
    Color::Indexed(117),
    Color::Indexed(33),
    Color::Indexed(200),
    Color::Indexed(82),
];

fn traffic(floors: usize, scale: f64) -> Box<dyn Traffic> {
    let lull = Box::new(traffic::Random::new(floors, vec![floors as f64], vec![floors as f64], scale));
    let spike = Box::new(traffic::Random::new(floors, vec![floors as f64], vec![floors as f64], 10.0 * scale));
    Box::new(traffic::Cycle::new(vec![lull, spike], vec![60_000, 10_000]))
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();
    let b = args.building();
    let floors = b.num_floors();

    let quantiles = [0.5, 0.95, 0.99, 0.999];
    let mut q_idx = 0;
    let mut vis_idx = 0;

    let mut controls = Controls::default();

    let mut sims: Vec<Simulation> = args
        .policies
        .iter()
        .map(|name| {
            let t = traffic(floors, controls.traffic_scale());
            Simulation::new(b.clone(), policies::new(name, &b), t, &controls)
        })
        .collect();

    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| {
            // Horizontal split: Left (Charts/Controls) and Right (Elevator Vis)
            let app_layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
                .split(f.area());

            // Vertical split for the left side: Controls, Throughput, and Latency
            let left_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),      // Controls
                    Constraint::Percentage(50), // Throughput Comparison
                    Constraint::Percentage(50), // Latency Time Series
                ])
                .split(app_layout[0]);

            // 1. Controls Panel
            f.render_widget(
                Paragraph::new(controls.to_string()).block(Block::default().borders(Borders::ALL).title("Simulation Controls")),
                left_layout[0],
            );

            // 2. Elevator Visualization Panel (Full Height on Right)
            let vis_sim = &sims[vis_idx];
            let mut vis_lines = Vec::new();
            let tpf = vis_sim.building.time_per_floor();

            for f_idx in (0..floors).rev() {
                let mut line_spans = vec![Span::raw(format!("{:>2} ", f_idx))];
                for (e_idx, e) in vis_sim.building.elevators.iter().enumerate() {
                    let color = ELEV_COLORS[e_idx % ELEV_COLORS.len()];
                    let pos_f = e.pos as f64 / tpf as f64;
                    if (pos_f - f_idx as f64).abs() <= 0.25 {
                        let elevator_icon = format!("[{}] ", e.passengers.len());
                        line_spans.push(Span::styled(elevator_icon, Style::default().fg(color)));
                    } else {
                        line_spans.push(Span::styled(" ║  ", Style::default().fg(Color::White)));
                    }
                }
                line_spans.push(Span::raw(" "));
                for e_idx in 0..vis_sim.building.elevators.len() {
                    let color = ELEV_COLORS[e_idx % ELEV_COLORS.len()];
                    let count = vis_sim.building.waiting_for_elevator(f_idx, e_idx);
                    if count > 0 {
                        line_spans.push(Span::styled("☺".repeat(count), Style::default().fg(color)));
                    }
                }
                vis_lines.push(Line::from(line_spans));

                if f_idx > 0 {
                    let mut spacer_spans = vec![Span::raw("   ")];
                    for (e_idx, e) in vis_sim.building.elevators.iter().enumerate() {
                        let color = ELEV_COLORS[e_idx % ELEV_COLORS.len()];
                        let pos_f = e.pos as f64 / tpf as f64;
                        if pos_f < f_idx as f64 - 0.25 && pos_f > f_idx as f64 - 0.75 {
                            let elevator_icon = format!("[{}] ", e.passengers.len());
                            spacer_spans.push(Span::styled(elevator_icon, Style::default().fg(color)));
                        } else {
                            spacer_spans.push(Span::styled(" ║  ", Style::default().fg(Color::White)));
                        }
                    }
                    vis_lines.push(Line::from(spacer_spans));
                }
            }
            f.render_widget(
                Paragraph::new(vis_lines).block(Block::default().borders(Borders::ALL).title(Span::styled(
                    format!("Visualization ({})", args.policies[vis_idx]),
                    Style::default().add_modifier(Modifier::BOLD),
                ))),
                app_layout[1],
            );

            // Combined Throughput Chart with Panning and Axes
            let chart_area = left_layout[1];
            let max_data_points = chart_area.width.saturating_sub(10) as usize;
            for sim in &mut sims {
                sim.stats.trim(max_data_points);
            }
            let history_len = sims[0].stats.len();

            let mut global_max_y = 5.0f64;
            let mut datasets_data = Vec::with_capacity(sims.len());

            for sim in &sims {
                let history = sim.stats.throughput_history();
                let points: Vec<(f64, f64)> = history
                    .iter()
                    .enumerate()
                    .filter_map(|(i, &v)| {
                        let x = i as f64 + max_data_points as f64 - history_len as f64;
                        if x >= 0.0 && x < max_data_points as f64 {
                            let val = v as f64;
                            if val > global_max_y {
                                global_max_y = val;
                            }
                            Some((x, val))
                        } else {
                            None
                        }
                    })
                    .collect();
                datasets_data.push(points);
            }

            let datasets: Vec<Dataset> = (0..sims.len())
                .map(|i| {
                    let color = POLICY_COLORS[i & POLICY_COLORS.len()];
                    let name = args.policies[i].as_str();
                    Dataset::default()
                        .name(name)
                        .marker(symbols::Marker::Octant)
                        .graph_type(GraphType::Line)
                        .style(Style::default().fg(color))
                        .data(&datasets_data[i])
                })
                .collect();

            let start = sims[0].stats.start / 1000;
            let end = sims[0].stats.end() / 1000;
            let chart = Chart::new(datasets)
                .block(Block::default().borders(Borders::ALL).title("Throughput"))
                .legend_position(Some(LegendPosition::TopLeft))
                .hidden_legend_constraints((Constraint::Min(0), Constraint::Min(0)))
                .x_axis(
                    Axis::default()
                        .title("Time (Seconds)")
                        .bounds([0.0, max_data_points as f64])
                        .labels(vec![Line::from(format!("{:<6}", start)), Line::from(format!("{:>6}", end))]),
                )
                .y_axis(Axis::default().title("Served").bounds([0.0, global_max_y * 1.2]).labels(vec![
                    Line::from(format!("{:<6}", "0")),
                    Line::from(format!("{:<6.0}", global_max_y)),
                ]));

            f.render_widget(chart, chart_area);

            // 3. Combined Latency Percentile Chart (Time Series)
            let hist_area = left_layout[2];
            let q_target = quantiles[q_idx];
            let mut hist_max_y = 0.1f64;
            let mut hist_datasets_data = Vec::with_capacity(sims.len());

            let max_lat_points = hist_area.width.saturating_sub(10) as usize;
            for sim in &mut sims {
                sim.stats.trim(max_lat_points);
            }
            let lat_history_len = sims[0].stats.len();

            for sim in &sims {
                let history = sim.stats.latency_history(q_target);
                let points: Vec<(f64, f64)> = history
                    .iter()
                    .enumerate()
                    .filter_map(|(i, &val)| {
                        let x = i as f64 + max_lat_points as f64 - lat_history_len as f64;
                        if x >= 0.0 && x < max_lat_points as f64 {
                            let val_sec = val / 1000.0;
                            if val_sec > hist_max_y {
                                hist_max_y = val_sec;
                            }
                            Some((x, val_sec))
                        } else {
                            None
                        }
                    })
                    .collect();
                hist_datasets_data.push(points);
            }

            let hist_datasets: Vec<Dataset> = (0..sims.len())
                .map(|i| {
                    let color = POLICY_COLORS[i & POLICY_COLORS.len()];
                    let name = args.policies[i].as_str();
                    Dataset::default()
                        .name(name)
                        .marker(symbols::Marker::Octant)
                        .graph_type(GraphType::Line)
                        .style(Style::default().fg(color))
                        .data(&hist_datasets_data[i])
                })
                .collect();

            let hist_chart = Chart::new(hist_datasets)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(format!("Latency P{}", q_target * 100.0)),
                )
                .legend_position(Some(LegendPosition::TopLeft))
                .hidden_legend_constraints((Constraint::Min(0), Constraint::Min(0)))
                .x_axis(
                    Axis::default()
                        .title("Time (Seconds)")
                        .bounds([0.0, max_lat_points as f64])
                        .labels(vec![Line::from(format!("{:<6}", start)), Line::from(format!("{:>6}", end))]),
                )
                .y_axis(
                    Axis::default()
                        .title("Latency (Seconds)")
                        .bounds([0.0, hist_max_y * 1.1])
                        .labels(vec![
                            Line::from(format!("{:<8}", "0.00s")),
                            Line::from(format!("{:<8.2}s", hist_max_y)),
                        ]),
                );

            f.render_widget(hist_chart, hist_area);
        })?;

        // Input Handling
        if event::poll(TICK_RATE)? {
            if let Event::Key(key) = event::read()? {
                // Only process the initial key press to avoid double-triggers on Windows
                if key.kind != event::KeyEventKind::Press {
                    continue;
                }

                match key.code {
                    KeyCode::Char('q') | KeyCode::Char('Q') => break,
                    KeyCode::Right => q_idx = (q_idx + 1) % quantiles.len(),
                    KeyCode::Left => q_idx = q_idx.saturating_sub(1),
                    KeyCode::Tab => vis_idx = (vis_idx + 1) % sims.len(),
                    _ => {}
                }

                controls.handle_input(key.code, &mut sims);
            }
        }

        // Tick Simulation
        let now = Instant::now();
        let elapsed = now.duration_since(last_tick);
        last_tick = now;
        let sim_step = elapsed.as_millis() as u64 * controls.speed();
        for sim in &mut sims {
            sim.tick(sim.building.prev_time() + sim_step);
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}
