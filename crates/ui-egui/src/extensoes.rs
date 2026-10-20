//! Angerô: botão "Extensões" na barra de cima (como Janela › Extensões do Illustrator).
//!
//! As extensões ficam na Suíte Angerô (`%LOCALAPPDATA%\Programs\Suite Angero\Suite\plugins`): a
//! lista vem do próprio `extensoes.py --listar` da Suíte e abrir uma extensão é um pedido ao
//! iniciador (porta 8790), que é ligado sem janela se ainda não estiver rodando. Sem a Suíte
//! instalada o botão não aparece.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use egui::{CornerRadius, Sense, Ui, vec2};

use crate::icons;

const PORTA_SUITE: u16 = 8790;

#[derive(Clone)]
struct Extensao {
    id: String,
    nome: String,
    suportada: bool,
}

#[derive(Clone)]
enum Lista {
    Carregando,
    Pronta(Vec<Extensao>),
    Erro,
}

fn lista() -> &'static Arc<Mutex<Lista>> {
    static L: OnceLock<Arc<Mutex<Lista>>> = OnceLock::new();
    L.get_or_init(|| Arc::new(Mutex::new(Lista::Carregando)))
}

/// A pasta da Suíte instalada (`…\Suite Angero\Suite`), ou `None` sem a Suíte.
fn pasta_suite() -> Option<&'static PathBuf> {
    static P: OnceLock<Option<PathBuf>> = OnceLock::new();
    P.get_or_init(|| {
        if !cfg!(windows) {
            return None;
        }
        let p = PathBuf::from(std::env::var_os("LOCALAPPDATA")?).join("Programs").join("Suite Angero").join("Suite");
        p.join("suite").join("extensoes.py").is_file().then_some(p)
    })
    .as_ref()
}

fn sem_janela(cmd: &mut Command) -> &mut Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    cmd
}

/// Lê a lista em segundo plano (o Python leva uns décimos de segundo para responder).
fn atualizar_lista(ctx: &egui::Context) {
    let Some(suite) = pasta_suite() else { return };
    *lista().lock().unwrap() = Lista::Carregando;
    let ctx = ctx.clone();
    std::thread::spawn(move || {
        let py = suite.join("python").join("python.exe");
        let py = if py.is_file() { py } else { PathBuf::from("python") };
        let saida = sem_janela(
            Command::new(py)
                .arg(suite.join("suite").join("extensoes.py"))
                .arg("--listar")
                .env("PYTHONIOENCODING", "utf-8")
                .current_dir(suite.join("suite")),
        )
        .output();
        let nova = saida
            .ok()
            .filter(|s| s.status.success())
            .and_then(|s| serde_json::from_slice::<Vec<serde_json::Value>>(&s.stdout).ok())
            .map(|itens| {
                itens
                    .iter()
                    .map(|e| Extensao {
                        id: e["id"].as_str().unwrap_or_default().to_string(),
                        nome: e["nome"].as_str().unwrap_or_default().to_string(),
                        suportada: e["suportada"].as_bool().unwrap_or(false),
                    })
                    .collect()
            })
            .map_or(Lista::Erro, Lista::Pronta);
        *lista().lock().unwrap() = nova;
        ctx.request_repaint();
    });
}

/// `POST /api/extensao` no iniciador da Suíte; `false` se ele não está rodando.
fn pedir_extensao(id: &str) -> bool {
    let corpo = serde_json::json!({ "id": id }).to_string();
    let addr = SocketAddr::from(([127, 0, 0, 1], PORTA_SUITE));
    let Ok(mut s) = TcpStream::connect_timeout(&addr, Duration::from_millis(500)) else { return false };
    s.set_read_timeout(Some(Duration::from_secs(30))).ok();
    let pedido = format!(
        "POST /api/extensao HTTP/1.1\r\nHost: 127.0.0.1:{PORTA_SUITE}\r\nContent-Type: application/json\r\nX-Suite: 1\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{corpo}",
        corpo.len()
    );
    if s.write_all(pedido.as_bytes()).is_err() {
        return false;
    }
    let mut resposta = String::new();
    s.read_to_string(&mut resposta).ok();
    true
}

/// Abre a extensão; liga o iniciador da Suíte (sem janela) se preciso.
fn abrir(id: String) {
    let Some(suite) = pasta_suite() else { return };
    std::thread::spawn(move || {
        if pedir_extensao(&id) {
            return;
        }
        let vbs = suite.join("suite").join("iniciar_suite.vbs");
        if sem_janela(Command::new("wscript.exe").arg(&vbs).arg("--sem-janela")).spawn().is_err() {
            return;
        }
        for _ in 0..40 {
            std::thread::sleep(Duration::from_millis(250));
            if pedir_extensao(&id) {
                return;
            }
        }
    });
}

/// Largura do botão com texto.
pub fn largura(ui: &Ui) -> f32 {
    let g = ui.painter().layout_no_wrap(tl!("Extensions").to_string(), egui::FontId::proportional(12.5), egui::Color32::WHITE);
    10.0 + 16.0 + 6.0 + g.size().x + 8.0 + 10.0 + 8.0
}

/// O botão (com texto, ou só o ícone quando falta espaço) e o seu menu. Não faz nada sem a Suíte.
pub fn botao(ui: &mut Ui, com_texto: bool) {
    if pasta_suite().is_none() {
        return;
    }
    let fonte = egui::FontId::proportional(12.5);
    let texto = tl!("Extensions").to_string();
    let galley = ui.painter().layout_no_wrap(texto.clone(), fonte, egui::Color32::WHITE);
    let tam = if com_texto { vec2(largura(ui), 26.0) } else { vec2(28.0, 26.0) };
    let (rect, resp) = ui.allocate_exact_size(tam, Sense::click());
    let cor = egui::Color32::from_rgb(0x3b, 0x4f, 0xa6);
    let cor = if resp.is_pointer_button_down_on() {
        cor.gamma_multiply(0.8)
    } else if resp.hovered() {
        egui::Color32::from_rgb(0x4a, 0x64, 0xc0)
    } else {
        cor
    };
    ui.painter().rect_filled(rect, CornerRadius::same(6), cor);
    if com_texto {
        let ic = egui::Rect::from_center_size(egui::pos2(rect.left() + 18.0, rect.center().y), vec2(16.0, 16.0));
        icons::paint(ui, "puzzle", ic, egui::Color32::WHITE);
        ui.painter().galley(egui::pos2(ic.right() + 6.0, rect.center().y - galley.size().y / 2.0), galley, egui::Color32::WHITE);
        let seta = egui::Rect::from_center_size(egui::pos2(rect.right() - 13.0, rect.center().y), vec2(10.0, 10.0));
        icons::paint(ui, "chevron-down", seta, egui::Color32::WHITE);
    } else {
        icons::paint(ui, "puzzle", egui::Rect::from_center_size(rect.center(), vec2(16.0, 16.0)), egui::Color32::WHITE);
    }
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, texto.clone()));
    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(texto);
    if resp.clicked() {
        atualizar_lista(ui.ctx());
    }
    egui::Popup::menu(&resp).show(|ui| {
        ui.set_min_width(220.0);
        match lista().lock().unwrap().clone() {
            Lista::Carregando => {
                ui.add_enabled(false, egui::Button::new(tl!("Loading…")));
            }
            Lista::Erro => {
                ui.add_enabled(false, egui::Button::new(tl!("Could not read the extensions")));
            }
            Lista::Pronta(itens) if itens.is_empty() => {
                ui.add_enabled(false, egui::Button::new(tl!("No extensions installed")));
            }
            Lista::Pronta(itens) => {
                for ext in itens {
                    if ext.suportada {
                        if ui.button(&ext.nome).clicked() {
                            abrir(ext.id.clone());
                            ui.close();
                        }
                    } else {
                        ui.add_enabled(false, egui::Button::new(format!("{} {}", ext.nome, tl!("(not adapted yet)"))));
                    }
                }
            }
        }
        ui.separator();
        if ui.button(tl!("Open extensions folder")).clicked() {
            if let Some(suite) = pasta_suite() {
                Command::new("explorer.exe").arg(suite.join("plugins")).spawn().ok();
            }
            ui.close();
        }
    });
}
