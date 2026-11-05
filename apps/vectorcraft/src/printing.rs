//! Desktop printer transport. VectorCraft composes every page and copy before submission;
//! craft-print owns discovery and spooling. Windows 7 keeps its viewer-based transport because
//! Windows.Data.Pdf is unavailable there.
#[cfg(not(all(windows, any(feature = "windows7", target_vendor = "win7"))))]
use std::process::Command;
use vectorcraft_ui_egui::print::{PrintJob, PrintService, Printer};

#[cfg(all(windows, any(feature = "windows7", target_vendor = "win7")))]
mod windows7;
#[cfg(all(windows, any(feature = "windows7", target_vendor = "win7")))]
use windows7 as system;

pub struct SystemPrint;
impl PrintService for SystemPrint {
    fn printers(&mut self) -> Vec<Printer> {
        system::printers()
    }
    fn print(&mut self, job: &PrintJob) -> Result<String, String> {
        let result = system::print(job);
        let to = job.printer.unwrap_or("the default printer");
        result.map(|note| format!("Sent “{}” to {to}{note}", job.title))
    }
    fn shared_print(&self) -> Option<vectorcraft_ui_egui::print::SharedPrintFn> {
        Some(std::sync::Arc::new(|job| SystemPrint.print(job)))
    }
    fn has_setup(&self) -> bool {
        true
    }
    fn setup(&mut self, printer: Option<&str>) -> Result<(), String> {
        system::setup(printer)
    }
}

#[cfg(not(all(windows, any(feature = "windows7", target_vendor = "win7"))))]
mod system {
    use super::*;
    pub fn printers() -> Vec<Printer> {
        // Angerô: no Windows a lista vem direto do spooler (EnumPrinters, milissegundos); o
        // craft_print sobe um PowerShell + CIM a cada abertura do Imprimir (1–3 s de tela parada).
        #[cfg(windows)]
        if let Some(lista) = super::windows_nativo::printers() {
            return lista;
        }
        craft_print::printers().into_iter().map(|p| Printer { name: p.name, default: p.default }).collect()
    }
    pub fn print(job: &PrintJob) -> Result<String, String> {
        // Copies, layout, marks and colour management are already in the PDF. Preserve its
        // physical sheet size and the queue's duplex default without printing copies twice.
        let settings = craft_print::Job { printer: job.printer.map(str::to_string), title: job.title.into(), ..Default::default() };
        craft_print::submit(job.pdf, &settings).map(|note| if note.is_empty() { note } else { format!(" ({note})") }).map_err(|e| e.to_string())
    }
    pub fn setup(printer: Option<&str>) -> Result<(), String> {
        #[cfg(windows)]
        {
            let _ = printer;
            Command::new("explorer").arg("ms-settings:printers").spawn().map(|_| ()).map_err(|e| e.to_string())
        }
        #[cfg(target_os = "macos")]
        {
            let _ = printer;
            Command::new("open")
                .arg("x-apple.systempreferences:com.apple.Print-Scan-Settings.extension")
                .spawn()
                .map(|_| ())
                .map_err(|e| e.to_string())
        }
        #[cfg(not(any(windows, target_os = "macos")))]
        {
            if Command::new("system-config-printer").spawn().is_ok() {
                return Ok(());
            }
            let url = printer.map_or_else(|| "http://localhost:631/printers/".to_string(), |p| format!("http://localhost:631/printers/{p}"));
            Command::new("xdg-open").arg(url).spawn().map(|_| ()).map_err(|e| e.to_string())
        }
    }
}

/// Angerô: as impressoras do Windows pelo spooler (EnumPrintersW nível 4 = só nome, lido do
/// registro, sem falar com cada impressora) e a padrão por GetDefaultPrinterW. `None` se o
/// spooler não responder: aí vale o caminho antigo do craft_print.
#[cfg(all(windows, not(any(feature = "windows7", target_vendor = "win7"))))]
mod windows_nativo {
    use vectorcraft_ui_egui::print::Printer;
    use windows::Win32::Graphics::Printing::{EnumPrintersW, GetDefaultPrinterW, PRINTER_ENUM_CONNECTIONS, PRINTER_ENUM_LOCAL, PRINTER_INFO_4W};
    use windows::core::{PCWSTR, PWSTR};

    pub fn printers() -> Option<Vec<Printer>> {
        let flags = PRINTER_ENUM_LOCAL | PRINTER_ENUM_CONNECTIONS;
        let (mut precisa, mut qtd) = (0u32, 0u32);
        // 1ª chamada só pergunta o tamanho (falha com ERROR_INSUFFICIENT_BUFFER, esperado).
        let _ = unsafe { EnumPrintersW(flags, PCWSTR::null(), 4, None, &mut precisa, &mut qtd) };
        let mut nomes = Vec::new();
        if precisa > 0 {
            // u64 para o buffer ficar alinhado para PRINTER_INFO_4W.
            let mut buf = vec![0u64; (precisa as usize).div_ceil(8)];
            let bytes = unsafe { std::slice::from_raw_parts_mut(buf.as_mut_ptr().cast::<u8>(), buf.len() * 8) };
            unsafe { EnumPrintersW(flags, PCWSTR::null(), 4, Some(bytes), &mut precisa, &mut qtd) }.ok()?;
            let itens = unsafe { std::slice::from_raw_parts(buf.as_ptr().cast::<PRINTER_INFO_4W>(), qtd as usize) };
            for i in itens {
                if let Ok(nome) = unsafe { i.pPrinterName.to_string() }
                    && !nome.is_empty()
                    && !nomes.contains(&nome)
                {
                    nomes.push(nome);
                }
            }
        }
        let padrao = padrao();
        Some(nomes.into_iter().map(|name| Printer { default: padrao.as_deref() == Some(name.as_str()), name }).collect())
    }

    fn padrao() -> Option<String> {
        let mut tam = 0u32;
        let _ = unsafe { GetDefaultPrinterW(None, &mut tam) };
        if tam == 0 {
            return None;
        }
        let mut buf = vec![0u16; tam as usize];
        if !unsafe { GetDefaultPrinterW(Some(PWSTR(buf.as_mut_ptr())), &mut tam) }.as_bool() {
            return None;
        }
        unsafe { PWSTR(buf.as_mut_ptr()).to_string() }.ok()
    }
}
