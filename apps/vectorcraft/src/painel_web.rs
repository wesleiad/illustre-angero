//! Angerô: a aba do Fluxogama no painel da direita (`vectorcraft_ui_egui::extensoes`).
//!
//! A página do plugin (servida pela Suíte Angerô) aparece num WebView2 — o motor do Edge que já vem
//! no Windows — como janela-filha da janela do Illustre, posta por cima do retângulo da aba. Fora
//! dessa aba (ou com um menu/diálogo do Illustre aberto por cima) ela fica escondida, sem fechar:
//! o login e o que estava aberto no Fluxogama continuam ao voltar para a aba.

#[cfg(all(windows, not(target_vendor = "win7")))]
pub use plataforma::PainelWeb;

#[cfg(not(all(windows, not(target_vendor = "win7"))))]
#[derive(Default)]
pub struct PainelWeb;

#[cfg(not(all(windows, not(target_vendor = "win7"))))]
impl PainelWeb {
    pub fn quadro(&mut self, _ctx: &egui::Context, _frame: &eframe::Frame) {}
}

#[cfg(all(windows, not(target_vendor = "win7")))]
mod plataforma {
    use std::path::PathBuf;

    use wry::dpi::{PhysicalPosition, PhysicalSize};
    use wry::{Rect, WebContext, WebView, WebViewBuilder};

    #[derive(Default)]
    pub struct PainelWeb {
        web: Option<WebView>,
        /// Fica vivo enquanto a página existe (pasta de dados do WebView2).
        _contexto: Option<WebContext>,
        /// Não abriu (WebView2 ausente ou com erro): não tenta de novo a cada quadro.
        falhou: bool,
        visivel: bool,
        caixa: Option<(i32, i32, u32, u32)>,
    }

    impl PainelWeb {
        /// Depois de desenhar o quadro: mostra, move ou esconde a página conforme a aba.
        pub fn quadro(&mut self, ctx: &egui::Context, frame: &eframe::Frame) {
            let Some((rect, url)) = vectorcraft_ui_egui::extensoes::painel_web(ctx) else {
                if self.visivel {
                    if let Some(web) = &self.web {
                        let _ = web.set_visible(false);
                        // o teclado volta para o Illustre (atalhos, texto)
                        let _ = web.focus_parent();
                    }
                    self.visivel = false;
                }
                return;
            };
            if self.web.is_none() && !self.falhou {
                self.criar(frame, &url);
            }
            let Some(web) = &self.web else { return };
            let ppp = ctx.pixels_per_point();
            let caixa = (
                (rect.left() * ppp).round() as i32,
                (rect.top() * ppp).round() as i32,
                (rect.width() * ppp).round().max(1.0) as u32,
                (rect.height() * ppp).round().max(1.0) as u32,
            );
            if self.caixa != Some(caixa) {
                let bounds = Rect { position: PhysicalPosition::new(caixa.0, caixa.1).into(), size: PhysicalSize::new(caixa.2, caixa.3).into() };
                let _ = web.set_bounds(bounds);
                self.caixa = Some(caixa);
            }
            if !self.visivel {
                let _ = web.set_visible(true);
                self.visivel = true;
            }
        }

        fn criar(&mut self, frame: &eframe::Frame, url: &str) {
            // Dados do WebView2 (login do Fluxogama, cache) na pasta do usuário, fora da pasta do programa.
            let pasta = std::env::var_os("LOCALAPPDATA").map(|p| PathBuf::from(p).join("Illustre-Angero").join("WebView2"));
            let mut contexto = WebContext::new(pasta);
            let feito = WebViewBuilder::new_with_web_context(&mut contexto)
                .with_url(url)
                .with_visible(false)
                .with_focused(false)
                .with_background_color((0x53, 0x53, 0x53, 0xff))
                .build_as_child(frame);
            match feito {
                Ok(web) => {
                    self.web = Some(web);
                    self._contexto = Some(contexto);
                }
                Err(e) => {
                    log::error!("Fluxogama: não abriu o WebView2: {e}");
                    self.falhou = true;
                }
            }
        }
    }
}
