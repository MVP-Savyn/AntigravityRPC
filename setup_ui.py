"""Ventana de configuración inicial y preferencias con tkinter."""

from __future__ import annotations

import os
import tkinter as tk
from tkinter import messagebox
import webbrowser

from config_store import (
    ASSETS_DIR,
    AppConfig,
    is_startup_enabled,
    load_config,
    save_config,
    set_startup_enabled,
)


DISCORD_DEV_URL = "https://discord.com/developers/applications"


def open_config_dialog() -> AppConfig | None:
    """Abre la ventana de configuración y devuelve el AppConfig guardado o None si cancela."""
    cfg = load_config()
    result: dict[str, AppConfig | None] = {"cfg": None}

    root = tk.Tk()
    root.title("Antigravity Rich Presence - Configuración")
    root.configure(bg="#111318")
    root.resizable(False, False)

    ico_path = ASSETS_DIR / "antigravity-icon.ico"
    if ico_path.exists():
        try:
            root.iconbitmap(str(ico_path))
        except Exception:
            pass

    bg = "#111318"
    card_bg = "#191c24"
    fg = "#eef1f6"
    muted = "#9aa3b2"
    accent = "#4c8bf5"
    entry_bg = "#0d0f14"

    container = tk.Frame(root, bg=bg, padx=22, pady=18)
    container.pack(fill="both", expand=True)

    # Header
    header = tk.Frame(container, bg=bg)
    header.pack(fill="x", pady=(0, 12))

    png_path = ASSETS_DIR / "antigravity-icon.png"
    logo_ref = None
    if png_path.exists():
        try:
            img = tk.PhotoImage(file=str(png_path)).subsample(11, 11)
            logo_ref = img
            lbl_img = tk.Label(header, image=img, bg=bg)
            lbl_img.pack(side="left", padx=(0, 12))
        except Exception:
            pass

    title_box = tk.Frame(header, bg=bg)
    title_box.pack(side="left", fill="x")
    tk.Label(
        title_box,
        text="Antigravity Discord Rich Presence",
        font=("Segoe UI", 13, "bold"),
        fg=fg,
        bg=bg,
    ).pack(anchor="w")
    tk.Label(
        title_box,
        text="Muestra tu proyecto activo e icono oficial en Discord",
        font=("Segoe UI", 9),
        fg=muted,
        bg=bg,
    ).pack(anchor="w")

    # Instructions card
    guide = tk.Frame(container, bg=card_bg, padx=14, pady=12, highlightthickness=1, highlightbackground="#262b36")
    guide.pack(fill="x", pady=(0, 14))

    tk.Label(
        guide,
        text="Cómo obtener tu Application ID e icono (1 vez):",
        font=("Segoe UI", 9, "bold"),
        fg=fg,
        bg=card_bg,
    ).pack(anchor="w", pady=(0, 4))

    steps = (
        "1. Crea una app llamada 'Antigravity' en Discord Developer Portal.\n"
        "2. En 'Rich Presence -> Art Assets', sube el PNG de la carpeta assets/\n"
        "   con el nombre de asset 'antigravity'.\n"
        "3. Copia el 'Application ID' de 'General Information' y pégalo abajo."
    )
    tk.Label(
        guide,
        text=steps,
        justify="left",
        font=("Segoe UI", 9),
        fg=muted,
        bg=card_bg,
    ).pack(anchor="w")

    btn_links = tk.Frame(guide, bg=card_bg)
    btn_links.pack(fill="x", pady=(10, 0))

    tk.Button(
        btn_links,
        text="Abrir Discord Developer Portal",
        command=lambda: webbrowser.open(DISCORD_DEV_URL),
        bg="#262b36",
        fg=fg,
        activebackground="#323846",
        activeforeground=fg,
        relief="flat",
        padx=10,
        pady=5,
        cursor="hand2",
        font=("Segoe UI", 9),
    ).pack(side="left", padx=(0, 8))

    tk.Button(
        btn_links,
        text="Abrir carpeta del icono (assets)",
        command=lambda: os.startfile(str(ASSETS_DIR)),
        bg="#262b36",
        fg=fg,
        activebackground="#323846",
        activeforeground=fg,
        relief="flat",
        padx=10,
        pady=5,
        cursor="hand2",
        font=("Segoe UI", 9),
    ).pack(side="left")

    # Form fields
    form = tk.Frame(container, bg=bg)
    form.pack(fill="x", pady=(0, 10))

    tk.Label(form, text="Discord Application ID (Client ID):", fg=fg, bg=bg, font=("Segoe UI", 9, "bold")).pack(
        anchor="w"
    )
    var_client_id = tk.StringVar(value=cfg.client_id)
    entry_id = tk.Entry(
        form,
        textvariable=var_client_id,
        bg=entry_bg,
        fg=fg,
        insertbackground=fg,
        relief="flat",
        font=("Consolas", 10),
        highlightthickness=1,
        highlightbackground="#2b313d",
        highlightcolor=accent,
    )
    entry_id.pack(fill="x", ipady=6, pady=(4, 10))

    tk.Label(
        form,
        text="Nombre del Art Asset (o URL pública HTTPS del icono):",
        fg=fg,
        bg=bg,
        font=("Segoe UI", 9),
    ).pack(anchor="w")
    var_large_img = tk.StringVar(value=cfg.large_image or "antigravity")
    tk.Entry(
        form,
        textvariable=var_large_img,
        bg=entry_bg,
        fg=fg,
        insertbackground=fg,
        relief="flat",
        font=("Consolas", 10),
        highlightthickness=1,
        highlightbackground="#2b313d",
        highlightcolor=accent,
    ).pack(fill="x", ipady=5, pady=(4, 10))

    # Toggles
    var_show_title = tk.BooleanVar(value=cfg.show_conversation_title)
    var_reset_timer = tk.BooleanVar(value=cfg.reset_timer_on_project_change)
    var_fallback_proj = tk.BooleanVar(value=cfg.fallback_to_last_project)
    var_startup = tk.BooleanVar(value=is_startup_enabled())

    for text, var in [
        ("Mostrar archivo activo o título de conversación", var_show_title),
        ("Reiniciar contador de tiempo al cambiar de proyecto", var_reset_timer),
        ("En chats sin proyecto, mostrar el último proyecto abierto", var_fallback_proj),
        ("Iniciar automáticamente con Windows", var_startup),
    ]:
        tk.Checkbutton(
            form,
            text=text,
            variable=var,
            bg=bg,
            fg=fg,
            selectcolor=entry_bg,
            activebackground=bg,
            activeforeground=fg,
            font=("Segoe UI", 9),
        ).pack(anchor="w", pady=2)

    # Bottom actions
    actions = tk.Frame(container, bg=bg)
    actions.pack(fill="x", pady=(10, 0))

    def on_save() -> None:
        cid = var_client_id.get().strip()
        if not cid.isdigit() or len(cid) < 15:
            messagebox.showerror(
                "Client ID inválido",
                "Introduce un Application ID numérico válido de Discord (ej: 1234567890123456789).",
                parent=root,
            )
            return
        new_cfg = AppConfig(
            client_id=cid,
            large_image=var_large_img.get().strip() or "antigravity",
            large_text=cfg.large_text,
            poll_interval_sec=cfg.poll_interval_sec,
            show_conversation_title=var_show_title.get(),
            reset_timer_on_project_change=var_reset_timer.get(),
            fallback_to_last_project=var_fallback_proj.get(),
        )
        save_config(new_cfg)
        try:
            set_startup_enabled(var_startup.get())
        except Exception as exc:
            messagebox.showwarning(
                "Inicio con Windows",
                f"Se guardó la configuración, pero falló el registro de inicio automático:\n{exc}",
                parent=root,
            )
        result["cfg"] = new_cfg
        root.destroy()

    def on_cancel() -> None:
        root.destroy()

    tk.Button(
        actions,
        text="Guardar y Conectar",
        command=on_save,
        bg=accent,
        fg="#ffffff",
        activebackground="#3b78e7",
        activeforeground="#ffffff",
        relief="flat",
        padx=16,
        pady=7,
        cursor="hand2",
        font=("Segoe UI", 9, "bold"),
    ).pack(side="right")

    tk.Button(
        actions,
        text="Cancelar",
        command=on_cancel,
        bg="#262b36",
        fg=fg,
        activebackground="#323846",
        activeforeground=fg,
        relief="flat",
        padx=14,
        pady=7,
        cursor="hand2",
        font=("Segoe UI", 9),
    ).pack(side="right", padx=(0, 8))

    # Keep reference so Tk doesn't garbage-collect the image
    root._logo_ref = logo_ref  # type: ignore[attr-defined]
    root.update_idletasks()
    w, h = root.winfo_width(), root.winfo_height()
    sw, sh = root.winfo_screenwidth(), root.winfo_screenheight()
    root.geometry(f"+{(sw - w) // 2}+{(sh - h) // 2}")
    root.mainloop()
    return result["cfg"]


if __name__ == "__main__":
    open_config_dialog()
