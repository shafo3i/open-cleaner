//! Architectural modal dialogs: rule specifications, purge confirmation, updates, and consent.

use std::thread;

use egui::Context;

use crate::core::ui::format_bytes;
use crate::ui::app::SystemCleanerApp;
use crate::ui::conductor::WorkerResult;
use crate::ui::model::TargetStatus;
use crate::ui::theme::{self, Tokens};
use crate::ui::widgets;
use crate::ui::{bloat_view, uninstaller_view};
use crate::updater;

/// Active top-level modal dialog state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Dialog {
    RuleSpecification(String),
    PurgeConfirmation(Vec<String>),
}

impl Dialog {
    /// Unique memory ID key for egui layout caching across modal switches.
    pub(crate) fn key(&self) -> (&'static str, &str) {
        match self {
            Dialog::RuleSpecification(id) => ("rule_spec", id),
            Dialog::PurgeConfirmation(_) => ("purge_confirm", ""),
        }
    }
}

impl SystemCleanerApp {
    /// Renders all system modals and floating notification dialogs.
    pub(crate) fn render_dialogs(&mut self, ctx: &Context) {
        // 1. Top-level unified modal dialogs
        if let Some(dialog) = self.dialog.clone() {
            let key = dialog.key();
            match &dialog {
                Dialog::RuleSpecification(target_id) => {
                    if self.render_rule_spec_dialog(ctx, target_id, key) {
                        self.dialog = None;
                    }
                }
                Dialog::PurgeConfirmation(targets) => {
                    let (proceed, cancel) = self.render_purge_confirm_dialog(ctx, targets, key);
                    if cancel {
                        self.dialog = None;
                    } else if proceed {
                        let targets = targets.clone();
                        self.dialog = None;
                        self.run_purge(targets);
                    }
                }
            }
        }

        // 2. Complete Application Uninstall Modal
        if let Some((app, opts)) = uninstaller_view::render_uninstall_modal(
            ctx,
            &mut self.uninstaller_state,
            self.secure_wipe,
            self.dark_mode,
        ) {
            self.run_app_uninstall(app, opts);
        }

        // 3. Bloatware removal consent modal
        if let Some(true) =
            bloat_view::render_bloat_consent_modal(ctx, &mut self.bloat_state, self.dark_mode)
        {
            let create_restore = self.bloat_state.create_restore;
            self.run_bloat_removal(create_restore);
        }

        // 4. Update available toast notification
        self.render_update_toast(ctx);
    }

    /// Renders the target rule specification modal inspecting safe cleanup parameters.
    fn render_rule_spec_dialog(
        &mut self,
        ctx: &Context,
        target_id: &str,
        key: (&'static str, &str),
    ) -> bool {
        let mut close = ctx.input(|i| i.key_pressed(egui::Key::Escape));
        let t = Tokens::get(self.dark_mode);

        let screen_rect = ctx.screen_rect();
        egui::Area::new(egui::Id::new("rule_modal_scrim").with(key))
            .order(egui::Order::Middle)
            .fixed_pos(screen_rect.min)
            .show(ctx, |ui| {
                let (rect, resp) = ui.allocate_exact_size(screen_rect.size(), egui::Sense::click());
                ui.painter().rect_filled(
                    rect,
                    egui::CornerRadius::ZERO,
                    egui::Color32::from_black_alpha(if self.dark_mode { 165 } else { 95 }),
                );
                if resp.clicked() {
                    close = true;
                }
            });

        egui::Area::new(egui::Id::new("rule_modal_card").with(key))
            .order(egui::Order::Foreground)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                let modal_w = 460.0;
                egui::Frame::new()
                    .fill(t.bg)
                    .stroke(egui::Stroke::new(1.0_f32, t.line_strong))
                    .corner_radius(egui::CornerRadius::ZERO)
                    .show(ui, |ui| {
                        ui.set_width(modal_w);

                        if let Some(target) =
                            self.targets.iter().find(|target| target.id == target_id)
                        {
                            // 40px Top Strip Header (matching Grid Cards)
                            let (strip_rect, _) =
                                ui.allocate_exact_size(egui::vec2(modal_w, 40.0), egui::Sense::hover());
                            ui.allocate_new_ui(
                                egui::UiBuilder::new().max_rect(strip_rect),
                                |ui| {
                                    ui.horizontal_centered(|ui| {
                                        ui.add_space(16.0);
                                        widgets::tag(
                                            ui,
                                            target.number,
                                            target.color,
                                            egui::Color32::WHITE,
                                        );
                                        ui.add_space(10.0);
                                        widgets::hud(ui, "TARGET RULE SPECIFICATION", t.muted);
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                ui.add_space(12.0);
                                                if widgets::nav_link(
                                                    ui,
                                                    "CLOSE ×",
                                                    40.0,
                                                    false,
                                                    self.dark_mode,
                                                )
                                                .clicked()
                                                {
                                                    close = true;
                                                }
                                            },
                                        );
                                    });
                                },
                            );
                            ui.painter().hline(
                                strip_rect.x_range(),
                                strip_rect.bottom(),
                                t.hairline(),
                            );

                            // Body Content
                            egui::Frame::new()
                                .inner_margin(egui::Margin::same(20))
                                .show(ui, |ui| {
                                    ui.set_width(modal_w - 40.0);

                                    ui.horizontal(|ui| {
                                        ui.label(
                                            egui::RichText::new(target.name_prefix)
                                                .font(egui::FontId::new(18.0, theme::display()))
                                                .color(t.ink_strong),
                                        );
                                        ui.label(
                                            egui::RichText::new(target.name_suffix)
                                                .font(egui::FontId::new(18.0, theme::display()))
                                                .color(target.color),
                                        );
                                    });
                                    ui.add_space(2.0);
                                    ui.label(
                                        egui::RichText::new(target.tagline)
                                            .font(egui::FontId::new(
                                                12.5,
                                                egui::FontFamily::Proportional,
                                            ))
                                            .color(t.muted),
                                    );

                                    ui.add_space(16.0);

                                    // Sunken Specification Box
                                    egui::Frame::new()
                                        .fill(t.sunken)
                                        .stroke(t.hairline())
                                        .corner_radius(egui::CornerRadius::ZERO)
                                        .inner_margin(egui::Margin::same(14))
                                        .show(ui, |ui| {
                                            ui.set_width(modal_w - 68.0);
                                            ui.spacing_mut().item_spacing.y = 8.0;

                                            ui.horizontal(|ui| {
                                                widgets::hud(ui, "RULE ID", t.faint);
                                                ui.with_layout(
                                                    egui::Layout::right_to_left(
                                                        egui::Align::Center,
                                                    ),
                                                    |ui| {
                                                        ui.label(
                                                            egui::RichText::new(target.id)
                                                                .font(egui::FontId::new(
                                                                    11.5,
                                                                    theme::mono_bold(),
                                                                ))
                                                                .color(t.ink_strong),
                                                        );
                                                    },
                                                );
                                            });

                                            ui.horizontal(|ui| {
                                                widgets::hud(ui, "CATEGORY", t.faint);
                                                ui.with_layout(
                                                    egui::Layout::right_to_left(
                                                        egui::Align::Center,
                                                    ),
                                                    |ui| {
                                                        let cat_label = match target.category {
                                                            "system" => "SYSTEM",
                                                            "browsers" => "BROWSERS",
                                                            "apps" => "APPS",
                                                            "dev" => "DEV & AI",
                                                            "graphics" => "GRAPHICS",
                                                            "privacy" => "PRIVACY",
                                                            "registry" => "REGISTRY",
                                                            other => other,
                                                        };
                                                        ui.label(
                                                            egui::RichText::new(cat_label)
                                                                .font(egui::FontId::new(
                                                                    11.0,
                                                                    theme::mono_bold(),
                                                                ))
                                                                .color(target.color),
                                                        );
                                                    },
                                                );
                                            });

                                            ui.horizontal(|ui| {
                                                widgets::hud(ui, "CONFIGURED SCOPE", t.faint);
                                                ui.with_layout(
                                                    egui::Layout::right_to_left(
                                                        egui::Align::Center,
                                                    ),
                                                    |ui| {
                                                        ui.label(
                                                            egui::RichText::new(target.scope)
                                                                .font(egui::FontId::new(
                                                                    11.0,
                                                                    theme::mono(),
                                                                ))
                                                                .color(t.muted),
                                                        );
                                                    },
                                                );
                                            });

                                            ui.horizontal(|ui| {
                                                widgets::hud(ui, "DETECTED FOOTPRINT", t.faint);
                                                ui.with_layout(
                                                    egui::Layout::right_to_left(
                                                        egui::Align::Center,
                                                    ),
                                                    |ui| {
                                                        let meta = format!(
                                                            "{} · {}",
                                                            target.count, target.size
                                                        );
                                                        ui.label(
                                                            egui::RichText::new(meta)
                                                                .font(egui::FontId::new(
                                                                    11.5,
                                                                    theme::mono_bold(),
                                                                ))
                                                                .color(t.accent_ink),
                                                        );
                                                    },
                                                );
                                            });
                                        });

                                    ui.add_space(14.0);

                                    let safety_note = match target.status {
                                        TargetStatus::Guarded => {
                                            "GUARDED: Registry entries are strictly verified against disk paths before removal."
                                        }
                                        _ if target.id.starts_with("rule_") => {
                                            "OPT-IN RULE: Loaded from the embedded safe-rule catalog. Root folders are preserved."
                                        }
                                        _ => {
                                            "VERIFIED TARGET: Active OS and application handles are automatically bypassed."
                                        }
                                    };
                                    ui.label(
                                        egui::RichText::new(safety_note)
                                            .font(egui::FontId::new(11.0, theme::mono()))
                                            .color(t.faint),
                                    );

                                    ui.add_space(18.0);

                                    if widgets::button(
                                        ui,
                                        "CLOSE SPECIFICATION",
                                        widgets::ButtonKind::Secondary,
                                        modal_w - 40.0,
                                        self.dark_mode,
                                    )
                                    .clicked()
                                    {
                                        close = true;
                                    }
                                });
                        }
                    });
            });

        close
    }

    /// Renders the confirmation modal before purging queued targets.
    fn render_purge_confirm_dialog(
        &mut self,
        ctx: &Context,
        targets: &[String],
        key: (&'static str, &str),
    ) -> (bool, bool) {
        let mut proceed = false;
        let mut cancel = ctx.input(|i| i.key_pressed(egui::Key::Escape));
        let t = Tokens::get(self.dark_mode);

        let screen_rect = ctx.screen_rect();
        egui::Area::new(egui::Id::new("purge_modal_scrim").with(key))
            .order(egui::Order::Middle)
            .fixed_pos(screen_rect.min)
            .show(ctx, |ui| {
                let (rect, resp) = ui.allocate_exact_size(screen_rect.size(), egui::Sense::click());
                ui.painter().rect_filled(
                    rect,
                    egui::CornerRadius::ZERO,
                    egui::Color32::from_black_alpha(if self.dark_mode { 165 } else { 95 }),
                );
                if resp.clicked() {
                    cancel = true;
                }
            });

        let single_target = if targets.len() == 1 {
            self.targets.iter().find(|t| t.id == targets[0])
        } else {
            None
        };
        let selected_bytes: u64 = self
            .targets
            .iter()
            .filter(|t| targets.iter().any(|id| id == &t.id) && t.category != "registry")
            .map(|t| t.raw_bytes)
            .sum();

        egui::Area::new(egui::Id::new("purge_modal_card").with(key))
            .order(egui::Order::Foreground)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                let modal_w = 460.0;
                egui::Frame::new()
                    .fill(t.bg)
                    .stroke(egui::Stroke::new(1.0_f32, t.line_strong))
                    .corner_radius(egui::CornerRadius::ZERO)
                    .show(ui, |ui| {
                        ui.set_width(modal_w);

                        // 40px Top Strip Header (matching Grid Cards)
                        let (strip_rect, _) =
                            ui.allocate_exact_size(egui::vec2(modal_w, 40.0), egui::Sense::hover());
                        let tag_color = single_target.map(|st| st.color).unwrap_or(t.bad);
                        let tag_label = single_target
                            .map(|st| st.number.to_string())
                            .unwrap_or_else(|| format!("{:02}", targets.len()));

                        ui.allocate_new_ui(
                            egui::UiBuilder::new().max_rect(strip_rect),
                            |ui| {
                                ui.horizontal_centered(|ui| {
                                    ui.add_space(16.0);
                                    widgets::tag(
                                        ui,
                                        &tag_label,
                                        tag_color,
                                        egui::Color32::WHITE,
                                    );
                                    ui.add_space(10.0);
                                    widgets::hud(ui, "CONFIRM PURGE EXECUTION", t.muted);
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            ui.add_space(12.0);
                                            if widgets::nav_link(
                                                ui,
                                                "ESC · CANCEL ×",
                                                40.0,
                                                false,
                                                self.dark_mode,
                                            )
                                            .clicked()
                                            {
                                                cancel = true;
                                            }
                                        },
                                    );
                                });
                            },
                        );
                        ui.painter().hline(
                            strip_rect.x_range(),
                            strip_rect.bottom(),
                            t.hairline(),
                        );

                        // Body Content
                        egui::Frame::new()
                            .inner_margin(egui::Margin::same(20))
                            .show(ui, |ui| {
                                ui.set_width(modal_w - 40.0);

                                // Two-tone Display Title
                                ui.horizontal(|ui| {
                                    if let Some(st) = single_target {
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "Purge {}",
                                                st.name_prefix
                                            ))
                                            .font(egui::FontId::new(18.0, theme::display()))
                                            .color(t.ink_strong),
                                        );
                                        ui.label(
                                            egui::RichText::new(st.name_suffix)
                                                .font(egui::FontId::new(18.0, theme::display()))
                                                .color(st.color),
                                        );
                                    } else {
                                        ui.label(
                                            egui::RichText::new("Purge")
                                                .font(egui::FontId::new(18.0, theme::display()))
                                                .color(t.ink_strong),
                                        );
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "{} Selected Targets",
                                                targets.len()
                                            ))
                                            .font(egui::FontId::new(18.0, theme::display()))
                                            .color(t.bad),
                                        );
                                    }
                                });

                                ui.add_space(2.0);
                                let subtitle = single_target
                                    .map(|st| st.tagline)
                                    .unwrap_or("Permanent removal of verified cache, log, and privacy artifacts.");
                                ui.label(
                                    egui::RichText::new(subtitle)
                                        .font(egui::FontId::new(
                                            12.5,
                                            egui::FontFamily::Proportional,
                                        ))
                                        .color(t.muted),
                                );

                                ui.add_space(16.0);

                                // Sunken Telemetry Summary Box
                                egui::Frame::new()
                                    .fill(t.sunken)
                                    .stroke(t.hairline())
                                    .corner_radius(egui::CornerRadius::ZERO)
                                    .inner_margin(egui::Margin::same(14))
                                    .show(ui, |ui| {
                                        ui.set_width(modal_w - 68.0);
                                        ui.spacing_mut().item_spacing.y = 8.0;

                                        ui.horizontal(|ui| {
                                            widgets::hud(ui, "TARGET SCOPE", t.faint);
                                            ui.with_layout(
                                                egui::Layout::right_to_left(
                                                    egui::Align::Center,
                                                ),
                                                |ui| {
                                                    let scope_str = single_target
                                                        .map(|st| st.scope.to_string())
                                                        .unwrap_or_else(|| {
                                                            format!(
                                                                "{} MODULES QUEUED",
                                                                targets.len()
                                                            )
                                                        });
                                                    ui.label(
                                                        egui::RichText::new(scope_str)
                                                            .font(egui::FontId::new(
                                                                11.0,
                                                                theme::mono(),
                                                            ))
                                                            .color(t.ink_strong),
                                                    );
                                                },
                                            );
                                        });

                                        ui.horizontal(|ui| {
                                            widgets::hud(ui, "ESTIMATED RECLAIM", t.faint);
                                            ui.with_layout(
                                                egui::Layout::right_to_left(
                                                    egui::Align::Center,
                                                ),
                                                |ui| {
                                                    let est_str = if let Some(st) = single_target
                                                    {
                                                        st.size.clone()
                                                    } else {
                                                        format_bytes(selected_bytes)
                                                    };
                                                    ui.label(
                                                        egui::RichText::new(est_str)
                                                            .font(egui::FontId::new(
                                                                11.5,
                                                                theme::mono_bold(),
                                                            ))
                                                            .color(t.good),
                                                    );
                                                },
                                            );
                                        });

                                        ui.horizontal(|ui| {
                                            widgets::hud(ui, "SANITIZATION MODE", t.faint);
                                            ui.with_layout(
                                                egui::Layout::right_to_left(
                                                    egui::Align::Center,
                                                ),
                                                |ui| {
                                                    let (mode_str, mode_col) =
                                                        if self.secure_wipe {
                                                            ("DOD 0x00 ZERO-FILL", t.warn)
                                                        } else {
                                                            (
                                                                "STANDARD + POSIX UNLINK",
                                                                t.accent_ink,
                                                            )
                                                        };
                                                    ui.label(
                                                        egui::RichText::new(mode_str)
                                                            .font(egui::FontId::new(
                                                                11.0,
                                                                theme::mono_bold(),
                                                            ))
                                                            .color(mode_col),
                                                    );
                                                },
                                            );
                                        });
                                    });

                                ui.add_space(14.0);

                                ui.label(
                                    egui::RichText::new(
                                        "Open application handles are safely skipped. Unlinked files cannot be recovered.",
                                    )
                                    .font(egui::FontId::new(11.0, theme::mono()))
                                    .color(t.faint),
                                );
                                if self.secure_wipe {
                                    ui.add_space(4.0);
                                    ui.label(
                                        egui::RichText::new(
                                            "NOTE: Zero-fill overwrite is best-effort on wear-leveled SSDs and cloud storage.",
                                        )
                                        .font(egui::FontId::new(10.5, theme::mono()))
                                        .color(t.warn),
                                    );
                                }

                                ui.add_space(18.0);

                                // Architectural Action Row
                                ui.horizontal(|ui| {
                                    if widgets::button(
                                        ui,
                                        "CANCEL",
                                        widgets::ButtonKind::Secondary,
                                        135.0,
                                        self.dark_mode,
                                    )
                                    .clicked()
                                    {
                                        cancel = true;
                                    }
                                    ui.add_space(8.0);
                                    if widgets::button(
                                        ui,
                                        "EXECUTE PURGE",
                                        widgets::ButtonKind::Primary,
                                        modal_w - 40.0 - 135.0 - 8.0,
                                        self.dark_mode,
                                    )
                                    .clicked()
                                    {
                                        proceed = true;
                                    }
                                });
                            });
                    });
            });

        (proceed, cancel)
    }

    /// Renders the update available toast card anchored to the top-right corner.
    fn render_update_toast(&mut self, ctx: &Context) {
        let Some(update) = self.pending_update.clone() else {
            return;
        };

        let t = Tokens::get(self.dark_mode);
        let dark = self.dark_mode;
        let mut dismiss = false;
        let mut update_now = false;

        egui::Area::new(egui::Id::new("update_toast"))
            .order(egui::Order::Foreground)
            .anchor(
                egui::Align2::RIGHT_TOP,
                egui::vec2(-16.0, theme::HEADER_H + 12.0),
            )
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(t.bg)
                    .stroke(egui::Stroke::new(1.0_f32, t.accent))
                    .inner_margin(egui::Margin::same(14))
                    .show(ui, |ui| {
                        ui.set_width(360.0);
                        ui.spacing_mut().item_spacing.y = 6.0;

                        widgets::eyebrow(ui, "UPDATE", "NEW VERSION AVAILABLE", dark);
                        ui.label(
                            egui::RichText::new(format!(
                                "Version {} is ready to install.",
                                update.version()
                            ))
                            .font(egui::FontId::new(13.0, theme::display()))
                            .color(t.ink_strong),
                        );

                        if self.updating {
                            widgets::hud(ui, "DOWNLOADING & INSTALLING UPDATE...", t.warn);
                        } else {
                            ui.horizontal(|ui| {
                                if widgets::button(
                                    ui,
                                    "DISMISS",
                                    widgets::ButtonKind::Secondary,
                                    0.0,
                                    dark,
                                )
                                .clicked()
                                {
                                    dismiss = true;
                                }
                                if widgets::button(
                                    ui,
                                    "UPDATE NOW",
                                    widgets::ButtonKind::Primary,
                                    0.0,
                                    dark,
                                )
                                .clicked()
                                {
                                    update_now = true;
                                }
                            });
                        }
                    });
            });

        if dismiss {
            self.pending_update = None;
        }
        if update_now && !self.updating {
            self.updating = true;
            let tx = self.tx.clone();
            thread::spawn(move || {
                let result = updater::perform_update();
                let (updated, message) = match result {
                    Ok(true) => (true, "update installed".to_string()),
                    Ok(false) => (false, "already up to date".to_string()),
                    Err(e) => (false, e),
                };
                let _ = tx.send(WorkerResult::UpdateFinished { updated, message });
            });
        }
    }
}
