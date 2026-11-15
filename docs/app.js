(() => {
  const I18N = {
    es: {
      'nav.simulator': 'Simulador',
      'nav.features': 'Características',
      'nav.download': 'Descargar',
      'nav.docs': 'Documentación',
      'hero.eyebrow': 'Windows 10/11 · Rust Nativo',
      'hero.titleStart': 'Tu trabajo en',
      'hero.titleEnd': ', vivo en Discord.',
      'hero.subtitle': 'Muestra tu proyecto activo, rama Git, tiempo invertido e interacciones reales. Ligero y sin procesos en segundo plano.',
      'hero.downloadSetup': 'Descargar',
      'hero.repoBtn': 'Repositorio',
      'sim.liveBadge': 'EN VIVO',
      'sim.playingHeading': 'Jugando',
      'sim.controlsTitle': 'Simulador de ajustes',
      'sim.freeChat': 'Chat libre',
      'sim.optStats': 'Tiempo invertido',
      'sim.optBranch': 'Rama Git',
      'sim.optBusy': 'Estado del agente',
      'sim.optConvo': 'Chat actual',
      'sim.optRepo': 'Botón repositorio',
      'feat.statsTitle': 'Tiempo real por proyecto',
      'feat.statsDesc': 'Muestra días activos, horas totales e interacciones reales sin exponer tu código.',
      'feat.launchTitle': 'Sin procesos en 2º plano',
      'feat.launchDesc': 'Inicia y se cierra automáticamente junto con Antigravity desde su acceso directo.',
      'feat.trayTitle': 'Tray minimalista ES / EN',
      'feat.trayDesc': 'Un clic para configurar o cerrar el RPC al instante. Preconfigurado listo para usar.',
      'docsBanner.title': '¿Quieres ver cómo funciona por dentro?',
      'docsBanner.sub': 'Consulta la guía de instalación, el modo --launch y las opciones en la documentación.',
      'docsBanner.cta': 'Abrir documentación',
      'dl.setupTitle': 'Descargar AntigravityRPC',
      'dl.setupDesc': 'Instalador nativo en Rust sin dependencias externas. Incluye el acceso directo para abrir Antigravity y RPC juntos.',
      'dl.copyHash': 'Copiar',
      'toast.copied': 'SHA-256 copiado al portapapeles',
      'toast.download': 'Iniciando descarga:',
      'docs.backHome': 'Inicio',
      'docs.toc': 'Contenido',
      'docs.navInstall': 'Instalación',
      'docs.navLaunch': 'Modo --launch y Tray',
      'docs.navArch': 'Telemetría local',
      'docs.navGit': 'Rama Git y repositorio',
      'docs.navConfig': 'Configuración',
      'docs.kicker': 'DOCUMENTACIÓN · V1.0.0',
      'docs.title': 'Guía técnica de AntigravityRPC',
      'docs.subtitle': 'Todo lo necesario para instalar, entender cómo se calculan tus estadísticas y personalizar tu presencia en Discord.',
      'docs.installTitle': '1. Instalación',
      'docs.installDesc': 'Ejecuta AntigravityRichPresenceSetup.exe (Windows 10/11 x64). El instalador incluye todo lo necesario en un único binario Rust nativo sin dependencias externas ni runtimes adicionales.',
      'docs.installItem1': 'Crea el acceso directo Antigravity (con Discord RPC) en el menú Inicio.',
      'docs.installItem2': 'Preconfigurado de fábrica con el Client ID oficial (1571349694016266300) y el icono de Antigravity.',
      'docs.installItem3': 'Si actualizas una versión abierta, el instalador cierra la instancia anterior automáticamente.',
      'docs.launchTitle': '2. Modo Launcher (--launch) y Bandeja del sistema',
      'docs.launchDesc': 'El acceso directo Antigravity (con Discord RPC) ejecuta AntigravityRichPresence.exe --launch. De este modo abre Antigravity, monitoriza sus ventanas y termina limpiamente en cuanto cierras Antigravity, evitando procesos residentes en segundo plano.',
      'docs.trayItem1': 'Clic izquierdo en el icono del tray: abre la ventana minimalista de configuración.',
      'docs.trayItem2': 'Clic derecho en el icono del tray: abre menú rápido con Configuración y Cerrar RPC.',
      'docs.trayItem3': 'Recuperación automática del icono del tray si Windows reinicia explorer.exe.',
      'docs.archTitle': '3. Telemetría 100% local y cálculo de estadísticas',
      'docs.archDesc': 'AntigravityRPC no envía datos a ningún servidor ni lee tu código fuente. Extrae metadatos locales en tu equipo y comunica directamente con Discord por \\\\.\\pipe\\discord-ipc-0:',
      'docs.archItem1': 'Proyecto y ventana activa: annotations/*.pbtxt, app_storage.json, projects/*.json y conversation_summaries.db.',
      'docs.archItem2': 'Días activos, horas e interacciones: agrega todas las conversaciones del proyecto desde transcript.jsonl contando días naturales distintos, horas reales e interacciones de usuario (USER_INPUT).',
      'docs.archItem3': 'Formato mostrado: Invertido: 30d | 74h | 413 interacciones (o en inglés Invested: 30d | 74h | 413 interactions). Si estás en un chat libre sin proyecto, muestra la conversación actual.',
      'docs.gitTitle': '4. Rama Git y botón Ver repositorio',
      'docs.gitDesc': 'Si la carpeta del proyecto es un repositorio Git, lee .git/HEAD para añadir la rama activa junto al proyecto y normaliza remotos SSH/HTTPS de .git/config para mostrar el botón interactivo Ver repositorio en Discord.',
      'docs.configTitle': '5. Opciones de configuración',
      'docs.configDesc': 'Los ajustes se guardan automáticamente en %APPDATA%\\AntigravityRichPresence\\config.json y se aplican al instante:',
      'docs.thOpt': 'Opción',
      'docs.thDef': 'Por defecto',
      'docs.thDesc': 'Descripción',
      'docs.rowLang': 'Idioma de Discord y de la interfaz (es / en).',
      'docs.rowStats': 'Muestra días, horas invertidas e interacciones del proyecto.',
      'docs.rowConvo': 'Incluye el título de la conversación actual dentro de un proyecto.',
      'docs.rowGit': 'Muestra la rama Git activa junto al nombre del proyecto.',
      'docs.rowBusy': 'Badge indicando si el agente está trabajando o listo.',
      'docs.rowRepo': 'Añade el botón Ver repositorio cuando hay remoto HTTPS/SSH.',
    },
    en: {
      'nav.simulator': 'Simulator',
      'nav.features': 'Features',
      'nav.download': 'Download',
      'nav.docs': 'Docs',
      'hero.eyebrow': 'Windows 10/11 · Native Rust',
      'hero.titleStart': 'Your work in',
      'hero.titleEnd': ', live on Discord.',
      'hero.subtitle': 'Showcases your active project, Git branch, invested time and real interactions. Lightweight with zero background processes.',
      'hero.downloadSetup': 'Download',
      'hero.repoBtn': 'Repository',
      'sim.liveBadge': 'LIVE',
      'sim.playingHeading': 'Playing',
      'sim.controlsTitle': 'Settings simulator',
      'sim.freeChat': 'Free chat',
      'sim.optStats': 'Invested time',
      'sim.optBranch': 'Git branch',
      'sim.optBusy': 'Agent status',
      'sim.optConvo': 'Current chat',
      'sim.optRepo': 'Repo button',
      'feat.statsTitle': 'Real-time project stats',
      'feat.statsDesc': 'Displays active days, total hours and real interactions without exposing your code.',
      'feat.launchTitle': 'Zero background processes',
      'feat.launchDesc': 'Launches and exits cleanly alongside Antigravity from its shortcut.',
      'feat.trayTitle': 'Minimal ES / EN tray',
      'feat.trayDesc': 'One click to configure or close RPC immediately. Ready out of the box.',
      'docsBanner.title': 'Want to see how it works under the hood?',
      'docsBanner.sub': 'Read the install guide, --launch mode lifecycle and options in the documentation.',
      'docsBanner.cta': 'Open docs',
      'dl.setupTitle': 'Download AntigravityRPC',
      'dl.setupDesc': 'Native Rust Windows installer with zero external dependencies. Includes the Antigravity + RPC launcher shortcut.',
      'dl.copyHash': 'Copy',
      'toast.copied': 'SHA-256 copied to clipboard',
      'toast.download': 'Starting download:',
      'docs.backHome': 'Home',
      'docs.toc': 'Contents',
      'docs.navInstall': 'Installation',
      'docs.navLaunch': '--launch Mode & Tray',
      'docs.navArch': 'Local telemetry',
      'docs.navGit': 'Git branch & repo',
      'docs.navConfig': 'Configuration',
      'docs.kicker': 'DOCUMENTATION · V1.0.0',
      'docs.title': 'AntigravityRPC Technical Guide',
      'docs.subtitle': 'Everything you need to install, understand how stats are calculated and customize your Discord presence.',
      'docs.installTitle': '1. Installation',
      'docs.installDesc': 'Run AntigravityRichPresenceSetup.exe (Windows 10/11 x64). Everything ships in a single native Rust binary with zero external dependencies or runtimes.',
      'docs.installItem1': 'Creates the Antigravity (with Discord RPC) shortcut in your Start Menu.',
      'docs.installItem2': 'Preconfigured out of the box with official Client ID (1571349694016266300) and Antigravity icon.',
      'docs.installItem3': 'Upgrading closes any running instance automatically before installing.',
      'docs.launchTitle': '2. Launcher Mode (--launch) & System Tray',
      'docs.launchDesc': 'The Antigravity (with Discord RPC) shortcut runs AntigravityRichPresence.exe --launch. It starts Antigravity, watches its windows, and terminates cleanly when you close Antigravity — leaving zero background processes.',
      'docs.trayItem1': 'Left-click tray icon: opens the minimal settings window.',
      'docs.trayItem2': 'Right-click tray icon: opens quick menu with Settings and Close RPC.',
      'docs.trayItem3': 'Automatic tray icon recovery if Windows restarts explorer.exe.',
      'docs.archTitle': '3. 100% Local Telemetry & Stat Calculation',
      'docs.archDesc': 'AntigravityRPC sends zero data to remote servers and never reads source code. It reads local metadata only and connects directly to Discord over \\\\.\\pipe\\discord-ipc-0:',
      'docs.archItem1': 'Active project & window: annotations/*.pbtxt, app_storage.json, projects/*.json and conversation_summaries.db.',
      'docs.archItem2': 'Active days, hours & interactions: aggregates every conversation in the project via transcript.jsonl counting distinct calendar days, active hours and USER_INPUT turns.',
      'docs.archItem3': 'Format shown: Invested: 30d | 74h | 413 interactions (or Invertido: 30d | 74h | 413 interacciones in Spanish). Outside projects, falls back to current chat title.',
      'docs.gitTitle': '4. Git Branch & View Repository Button',
      'docs.gitDesc': 'When a project is a Git repository, it reads .git/HEAD for the active branch and normalizes SSH/HTTPS remotes from .git/config to attach the View repository Discord button.',
      'docs.configTitle': '5. Configuration Options',
      'docs.configDesc': 'Preferences are saved automatically to %APPDATA%\\AntigravityRichPresence\\config.json and applied live:',
      'docs.thOpt': 'Option',
      'docs.thDef': 'Default',
      'docs.thDesc': 'Description',
      'docs.rowLang': 'Discord and UI language (es / en).',
      'docs.rowStats': 'Displays project active days, hours and interactions.',
      'docs.rowConvo': 'Includes the active conversation title inside a project.',
      'docs.rowGit': 'Displays active Git branch next to the project name.',
      'docs.rowBusy': 'Badge indicating whether the agent is working or ready.',
      'docs.rowRepo': 'Attaches the View repository button for HTTPS/SSH remotes.',
    }
  };

  const PROJECTS = {
    hollowdrive: {
      name: 'SAVIN-HOLLOWDRIVE',
      branch: 'main',
      days: 30,
      hours: 74,
      interactions: 413,
      convoEs: 'Rediseño del menú radial y shader',
      convoEn: 'Radial menu & shader refactor',
      hasRepo: true,
    },
    antigravityRpc: {
      name: 'antigravity-rich-presence',
      branch: 'feat/native-tray',
      days: 6,
      hours: 19,
      interactions: 164,
      convoEs: 'Landing page y empaquetado',
      convoEn: 'Landing page & installer build',
      hasRepo: true,
    },
    freeChat: {
      name: null,
      branch: null,
      days: 1,
      hours: 1,
      interactions: 18,
      convoEs: 'Explorando arquitectura ECS en Rust',
      convoEn: 'Exploring Rust ECS architecture',
      hasRepo: false,
    }
  };

  const state = {
    lang: localStorage.getItem('agrpc_lang') === 'en' ? 'en' : 'es',
    projectKey: 'hollowdrive',
    showStats: true,
    showBranch: true,
    showBusy: true,
    showConvo: false,
    showRepo: true,
    elapsedSeconds: 258,
  };

  const els = {
    langBtns: document.querySelectorAll('.lang-btn'),
    projectPills: document.querySelectorAll('.project-pill'),
    optStats: document.getElementById('opt-stats'),
    optBranch: document.getElementById('opt-branch'),
    optBusy: document.getElementById('opt-busy'),
    optConvo: document.getElementById('opt-convo'),
    optRepo: document.getElementById('opt-repo'),
    simDetails: document.getElementById('sim-details'),
    simState: document.getElementById('sim-state'),
    simArtBox: document.getElementById('sim-art-box'),
    simSmallBadge: document.getElementById('sim-small-badge'),
    simRepoBtn: document.getElementById('sim-repo-btn'),
    simRepoLabel: document.getElementById('sim-repo-label'),
    simElapsed: document.getElementById('sim-elapsed'),
    toast: document.getElementById('toast'),
  };

  let toastTimer = null;
  function showToast(msg) {
    if (!els.toast) return;
    els.toast.textContent = msg;
    els.toast.classList.add('is-visible');
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => {
      els.toast.classList.remove('is-visible');
    }, 2400);
  }

  function applyLanguage(lang) {
    state.lang = lang === 'en' ? 'en' : 'es';
    localStorage.setItem('agrpc_lang', state.lang);
    document.documentElement.lang = state.lang;
    const dict = I18N[state.lang];

    document.querySelectorAll('[data-i18n]').forEach((node) => {
      const key = node.getAttribute('data-i18n');
      if (dict[key]) node.textContent = dict[key];
    });

    els.langBtns.forEach((btn) => {
      btn.classList.toggle('active', btn.dataset.lang === state.lang);
    });

    renderSimulator();
  }

  function formatDuration(days, hours) {
    const parts = [];
    if (days > 0) parts.push(`${days}d`);
    if (hours > 0) parts.push(`${hours}h`);
    return parts.join(' | ');
  }

  function renderSimulator() {
    if (!els.simDetails || !els.simState) return;
    const proj = PROJECTS[state.projectKey] || PROJECTS.hollowdrive;
    const isEn = state.lang === 'en';
    const convo = isEn ? proj.convoEn : proj.convoEs;

    let line1 = '';
    if (proj.name) {
      line1 = state.showBranch && proj.branch ? `${proj.name} (${proj.branch})` : proj.name;
    } else {
      line1 = convo;
    }

    const statsParts = [];
    if (state.showStats) {
      const duration = formatDuration(proj.days, proj.hours);
      if (duration && proj.name) {
        statsParts.push(`${isEn ? 'Invested' : 'Invertido'}: ${duration}`);
      }
      statsParts.push(`${proj.interactions} ${isEn ? 'interactions' : 'interacciones'}`);
    }

    let line2 = '';
    if (proj.name && state.showConvo) {
      line2 = statsParts.length > 0 ? `${convo} · ${statsParts.join(' | ')}` : convo;
    } else if (statsParts.length > 0) {
      line2 = statsParts.join(' | ');
    } else {
      line2 = isEn ? 'Active session in Antigravity' : 'Sesión activa en Antigravity';
    }

    els.simDetails.textContent = line1;
    els.simState.textContent = line2;

    if (els.simArtBox && els.simSmallBadge) {
      const busyText = state.showBusy
        ? (isEn ? 'Working' : 'Trabajando')
        : (isEn ? 'Ready' : 'Listo');
      els.simArtBox.setAttribute('data-tooltip', `Antigravity · ${busyText}`);
      els.simSmallBadge.classList.toggle('is-ready', !state.showBusy);
    }

    if (els.simRepoBtn && els.simRepoLabel) {
      const showButton = state.showRepo && proj.hasRepo;
      els.simRepoBtn.classList.toggle('is-hidden', !showButton);
      els.simRepoLabel.textContent = isEn ? 'View repository' : 'Ver repositorio';
    }
  }

  if (els.simElapsed) {
    setInterval(() => {
      state.elapsedSeconds += 1;
      const mins = Math.floor(state.elapsedSeconds / 60);
      const secs = state.elapsedSeconds % 60;
      els.simElapsed.textContent = `${String(mins).padStart(2, '0')}:${String(secs).padStart(2, '0')}`;
    }, 1000);
  }

  els.langBtns.forEach((btn) => {
    btn.addEventListener('click', () => applyLanguage(btn.dataset.lang));
  });

  els.projectPills.forEach((pill) => {
    pill.addEventListener('click', () => {
      els.projectPills.forEach((p) => p.classList.remove('active'));
      pill.classList.add('active');
      state.projectKey = pill.dataset.project;
      renderSimulator();
    });
  });

  els.optStats?.addEventListener('change', (e) => { state.showStats = e.target.checked; renderSimulator(); });
  els.optBranch?.addEventListener('change', (e) => { state.showBranch = e.target.checked; renderSimulator(); });
  els.optBusy?.addEventListener('change', (e) => { state.showBusy = e.target.checked; renderSimulator(); });
  els.optConvo?.addEventListener('change', (e) => { state.showConvo = e.target.checked; renderSimulator(); });
  els.optRepo?.addEventListener('change', (e) => { state.showRepo = e.target.checked; renderSimulator(); });

  document.querySelectorAll('[data-copy-hash]').forEach((btn) => {
    btn.addEventListener('click', async () => {
      const hash = btn.getAttribute('data-copy-hash');
      try {
        await navigator.clipboard.writeText(hash);
        showToast(I18N[state.lang]['toast.copied']);
      } catch {
        showToast(hash);
      }
    });
  });

  document.querySelectorAll('[data-download-track]').forEach((link) => {
    link.addEventListener('click', () => {
      const file = link.getAttribute('href')?.split('/').pop() || 'AntigravityRPC';
      showToast(`${I18N[state.lang]['toast.download']} ${file}`);
    });
  });

  document.querySelectorAll('.spotlight-card').forEach((card) => {
    card.addEventListener('pointermove', (e) => {
      const rect = card.getBoundingClientRect();
      card.style.setProperty('--mx', `${e.clientX - rect.left}px`);
      card.style.setProperty('--my', `${e.clientY - rect.top}px`);
    });
  });

  const tiltWrapper = document.getElementById('rpc-tilt-card');
  const discordCard = tiltWrapper?.querySelector('.discord-card');
  if (tiltWrapper && discordCard && !window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
    tiltWrapper.addEventListener('pointermove', (e) => {
      const rect = tiltWrapper.getBoundingClientRect();
      const px = (e.clientX - rect.left) / rect.width - 0.5;
      const py = (e.clientY - rect.top) / rect.height - 0.5;
      const rx = (-py * 9).toFixed(2);
      const ry = (px * 10).toFixed(2);
      discordCard.style.transform = `rotateX(${rx}deg) rotateY(${ry}deg) translateY(-2px)`;
    });

    tiltWrapper.addEventListener('pointerleave', () => {
      discordCard.style.transform = 'rotateX(0deg) rotateY(0deg) translateY(0px)';
    });
  }

  if (!window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
    document.querySelectorAll('.magnetic').forEach((btn) => {
      const strength = parseFloat(btn.getAttribute('data-magnetic') || '0.18');
      btn.addEventListener('pointermove', (e) => {
        const rect = btn.getBoundingClientRect();
        const dx = (e.clientX - (rect.left + rect.width / 2)) * strength;
        const dy = (e.clientY - (rect.top + rect.height / 2)) * strength;
        btn.style.transform = `translate(${dx.toFixed(1)}px, ${dy.toFixed(1)}px)`;
      });
      btn.addEventListener('pointerleave', () => {
        btn.style.transform = '';
      });
    });
  }

  const revealEls = document.querySelectorAll('.reveal');
  if ('IntersectionObserver' in window) {
    const observer = new IntersectionObserver((entries) => {
      entries.forEach((entry) => {
        if (entry.isIntersecting) {
          entry.target.classList.add('in-view');
          observer.unobserve(entry.target);
        }
      });
    }, { threshold: 0.12 });
    revealEls.forEach((el) => observer.observe(el));
  } else {
    revealEls.forEach((el) => el.classList.add('in-view'));
  }

  const canvas = document.getElementById('gravity-canvas');
  const ctx = canvas?.getContext('2d');
  if (canvas && ctx && !window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
    const palette = ['66, 133, 244', '52, 168, 83', '251, 188, 5', '234, 67, 53', '180, 200, 255'];
    const pointer = { x: -9999, y: -9999, active: false };
    let width = 0;
    let height = 0;
    let particles = [];

    function resize() {
      const dpr = Math.min(window.devicePixelRatio || 1, 2);
      width = window.innerWidth;
      height = window.innerHeight;
      canvas.width = width * dpr;
      canvas.height = height * dpr;
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);

      const count = Math.min(72, Math.max(28, Math.floor((width * height) / 24000)));
      particles = Array.from({ length: count }, (_, idx) => ({
        x: Math.random() * width,
        y: Math.random() * height,
        vx: (Math.random() - 0.5) * 0.35,
        vy: -0.18 - Math.random() * 0.35,
        r: 1.2 + Math.random() * 2.1,
        color: palette[idx % palette.length],
        alpha: 0.25 + Math.random() * 0.55,
      }));
    }

    window.addEventListener('resize', resize);
    window.addEventListener('pointermove', (e) => {
      pointer.x = e.clientX;
      pointer.y = e.clientY;
      pointer.active = true;
    });
    window.addEventListener('pointerleave', () => {
      pointer.active = false;
    });

    function frame() {
      ctx.clearRect(0, 0, width, height);

      for (let i = 0; i < particles.length; i++) {
        const p = particles[i];

        if (pointer.active) {
          const dx = p.x - pointer.x;
          const dy = p.y - pointer.y;
          const distSq = dx * dx + dy * dy;
          if (distSq < 22500 && distSq > 4) {
            const dist = Math.sqrt(distSq);
            const force = (1 - dist / 150) * 0.85;
            p.vx += (dx / dist) * force * 0.14;
            p.vy += (dy / dist) * force * 0.14;
          }
        }

        p.vx *= 0.985;
        p.vy = p.vy * 0.985 - 0.004;
        if (p.vy < -0.75) p.vy = -0.75;

        p.x += p.vx;
        p.y += p.vy;

        if (p.y < -12) {
          p.y = height + 12;
          p.x = Math.random() * width;
        }
        if (p.x < -12) p.x = width + 12;
        if (p.x > width + 12) p.x = -12;

        ctx.beginPath();
        ctx.arc(p.x, p.y, p.r, 0, Math.PI * 2);
        ctx.fillStyle = `rgba(${p.color}, ${p.alpha})`;
        ctx.fill();

        for (let j = i + 1; j < particles.length; j++) {
          const q = particles[j];
          const lx = p.x - q.x;
          const ly = p.y - q.y;
          const lDist = lx * lx + ly * ly;
          if (lDist < 12100) {
            const opacity = (1 - Math.sqrt(lDist) / 110) * 0.12;
            ctx.beginPath();
            ctx.moveTo(p.x, p.y);
            ctx.lineTo(q.x, q.y);
            ctx.strokeStyle = `rgba(140, 180, 255, ${opacity.toFixed(3)})`;
            ctx.lineWidth = 1;
            ctx.stroke();
          }
        }
      }

      requestAnimationFrame(frame);
    }

    resize();
    requestAnimationFrame(frame);
  }

  applyLanguage(state.lang);
})();
