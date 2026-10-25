//! Native account panel; authentication is shared with the desktop launcher.
use super::pages::{Action, button};
use crate::account::{self, Account, Accounts, Secrets, SignIn, SignInState};
use horizon_client::ui_theme::{Face, Ui, color};

pub(super) struct Login {
    pub visible: bool,
    task: Option<SignIn>,
    state: Option<SignInState>,
    message: String,
}
impl Login {
    pub fn new() -> Self {
        Self {
            visible: false,
            task: None,
            state: None,
            message: String::new(),
        }
    }
    pub fn close(&mut self) {
        if let Some(task) = self.task.take() {
            task.cancel();
        }
        self.state = None;
        self.visible = false;
    }
    pub fn start(&mut self, id: &str) {
        self.close();
        self.visible = true;
        let id = id.trim();
        if id.len() != 36
            || !id.chars().enumerate().all(|(i, c)| {
                if [8, 13, 18, 23].contains(&i) {
                    c == '-'
                } else {
                    c.is_ascii_hexdigit()
                }
            })
        {
            self.message =
                "Register our Microsoft public client application first, then enter its client ID."
                    .into();
            return;
        }
        self.message.clear();
        self.task = Some(SignIn::start_with_client_id(Some(id.into())));
        self.state = Some(SignInState::Starting);
    }
    pub fn poll(&mut self) {
        if let Some(task) = &self.task {
            self.state = Some(task.state());
            if let Some(SignInState::Done(result)) = &self.state {
                self.message = match result {
                    Ok(a) => format!("Signed in as {}", a.name),
                    Err(e) => e.clone(),
                };
                self.task = None;
            }
        }
    }
    pub fn sign_out(&mut self) {
        self.close();
        self.visible = true;
        let mut accounts = Accounts::load();
        if let Some(a) = accounts.selected().cloned() {
            self.message = match accounts
                .remove(account::store(), &a.uuid)
                .and_then(|_| accounts.save())
            {
                Ok(()) => "Signed out. Offline play is available.".into(),
                Err(e) => e,
            };
        }
    }
    pub fn open_link(&mut self) {
        if let Some(SignInState::Code(code)) = &self.state
            && let Err(e) = crate::platform::open_login_url(&code.verification_uri)
        {
            self.message = e;
        }
    }
    pub fn session(id: &str) -> Result<Option<(Account, Secrets)>, String> {
        let mut accounts = Accounts::load();
        let Some(a) = accounts.selected().cloned() else {
            return Ok(None);
        };
        let secrets = account::store()
            .get(&a.uuid)?
            .and_then(|s| serde_json::from_str(&s).ok())
            .ok_or("Session ended. Sign in again, or sign out to play offline.")?;
        let (a, secrets) = account::fresh(&account::Endpoints::live(), id.trim(), &a, secrets)?;
        accounts.add(account::store(), a.clone(), &secrets)?;
        accounts.save()?;
        Ok(Some((a, secrets)))
    }
    pub fn draw(&self, ui: &mut Ui, targets: &mut Vec<([f32; 4], Action)>, id: &str) {
        targets.clear(); // Modal input must never reach the page beneath it.
        let w = (ui.w - 32.0).min(720.0);
        let h = (ui.h - 24.0).min(490.0);
        let x = (ui.w - w) / 2.0;
        let y = (ui.h - h) / 2.0;
        ui.rounded(x, y, w, h, 8.0, color::BG);
        ui.text(
            Face::Medium,
            24.0,
            x + 20.0,
            y + 18.0,
            color::TEXT,
            "Microsoft / Xbox",
        );
        let account = Accounts::load()
            .selected()
            .map(|a| format!("Minecraft profile: {}", a.name))
            .unwrap_or_else(|| {
                "Offline profile. A Minecraft: Java Edition license is required.".into()
            });
        ui.text(
            Face::Regular,
            14.0,
            x + 20.0,
            y + 58.0,
            color::MUTED,
            &account,
        );
        ui.text(
            Face::Regular,
            14.0,
            x + 20.0,
            y + 86.0,
            color::MUTED,
            "Public client ID of our own approved application:",
        );
        button(
            ui,
            targets,
            [x + 20.0, y + 110.0, w - 40.0, 42.0],
            if id.is_empty() {
                "Not configured - enter client ID"
            } else {
                id
            },
            Action::Field("client_id"),
            false,
        );
        let text = match &self.state {
            Some(SignInState::Starting) => "Requesting a sign-in code...".into(),
            Some(SignInState::Code(code)) => {
                format!("Open Microsoft sign-in and enter: {}", code.user_code)
            }
            _ => self.message.clone(),
        };
        // Wrap status and errors instead of drawing over the modal edge.
        let limit = ((w - 40.0) / 7.5).max(20.0) as usize;
        let mut line = String::new();
        let mut row = 0;
        for word in text.split_whitespace() {
            if line.len() + word.len() + 1 > limit {
                ui.text(
                    Face::Regular,
                    14.0,
                    x + 20.0,
                    y + 174.0 + row as f32 * 20.0,
                    color::TEXT,
                    &line,
                );
                row += 1;
                line.clear();
                if row >= 4 {
                    break;
                }
            }
            if !line.is_empty() {
                line.push(' ')
            }
            line.push_str(word);
        }
        if row < 4 {
            ui.text(
                Face::Regular,
                14.0,
                x + 20.0,
                y + 174.0 + row as f32 * 20.0,
                color::TEXT,
                &line,
            );
        }
        ui.text(Face::Regular,13.0,x+20.0,y+h-144.0,color::MUTED,"Tokens stay in memory for this app session. Passwords are entered in Microsoft's browser.");
        let half = (w - 52.0) / 2.0;
        button(
            ui,
            targets,
            [x + 20.0, y + h - 110.0, half, 42.0],
            if matches!(self.state, Some(SignInState::Code(_))) {
                "Open Microsoft sign-in"
            } else {
                "Sign in"
            },
            if matches!(self.state, Some(SignInState::Code(_))) {
                Action::LoginLink
            } else {
                Action::SignIn
            },
            true,
        );
        button(
            ui,
            targets,
            [x + 32.0 + half, y + h - 110.0, half, 42.0],
            "Sign out",
            Action::SignOut,
            false,
        );
        button(
            ui,
            targets,
            [x + 20.0, y + h - 58.0, w - 40.0, 42.0],
            "Close",
            Action::CloseLogin,
            false,
        );
    }
}
impl Drop for Login {
    fn drop(&mut self) {
        if let Some(task) = &self.task {
            task.cancel();
        }
    }
}
