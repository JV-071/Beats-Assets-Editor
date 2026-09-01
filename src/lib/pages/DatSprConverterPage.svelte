<script lang="ts">
  import { onMount } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { appState } from "../../stores/appState.svelte";
  import { assetsState } from "../../stores/assetsState.svelte";
  import { invoke } from "../../utils/invoke";
  import { COMMANDS } from "../../commands";
  import { showStatus } from "../../utils";
  import { translate } from "../../i18n";
  import { loadAppearancesForAssetsEditor } from "../../appearanceLoader";
  import { loadSpecialMeaningIds } from "../../specialMeaning";

  // ── Types ──────────────────────────────────────────────────────────────────

  interface SupportedLegacyVersion {
    id: number;
    name: string;
    dat_signature: number;
    spr_signature: number;
    structure: number;
    default_extended: boolean;
    default_frame_groups: boolean;
    default_improved_animations: boolean;
  }

  interface LegacyDetectedInfo {
    dat_signature: number;
    spr_signature: number;
    detected_version_id: number;
    detected_version_name: string;
    object_count: number;
    outfit_count: number;
    effect_count: number;
    missile_count: number;
    total_things: number;
    sprite_count: number;
    is_extended: boolean;
    suggested_transparency: boolean;
    suggested_frame_groups: boolean;
    suggested_improved_animations: boolean;
  }

  interface ConversionResult {
    success: boolean;
    output_dir: string;
    appearances_path: string;
    catalog_path: string;
    aec_path: string | null;
    object_count: number;
    outfit_count: number;
    effect_count: number;
    missile_count: number;
    sprites_converted: number;
    sheets_created: number;
    elapsed_ms: number;
  }

  // ── State ──────────────────────────────────────────────────────────────────

  let datPath = $state("");
  let sprPath = $state("");
  let outputDir = $state("");

  let versions = $state<SupportedLegacyVersion[]>([]);
  let selectedVersionId = $state<number>(0); // 0 = Auto-detect
  let detectedInfo = $state<LegacyDetectedInfo | null>(null);
  let isDetecting = $state(false);

  // Conversion Options
  let extendedSprites = $state(false);
  let transparency = $state(false);
  let frameGroups = $state(false);
  let improvedAnimations = $state(false);
  let exportAec = $state(true);
  let openAfterConvert = $state(true);
  let projectName = $state("converted_legacy_assets");

  // Conversion Execution State
  let isConverting = $state(false);
  let conversionProgressText = $state("");
  let conversionResult = $state<ConversionResult | null>(null);
  let errorMessage = $state<string | null>(null);

  // ── Lifecycle ─────────────────────────────────────────────────────────────

  onMount(async () => {
    try {
      versions = await invoke<SupportedLegacyVersion[]>(
        COMMANDS.GET_SUPPORTED_LEGACY_VERSIONS,
      );
    } catch (e) {
      console.error("Failed to load supported legacy versions:", e);
    }
  });

  // ── File Selection ─────────────────────────────────────────────────────────

  async function browseDatFile() {
    try {
      const selected = await open({
        multiple: false,
        filters: [{ name: "Tibia DAT", extensions: ["dat"] }],
      });
      if (typeof selected === "string" && selected) {
        datPath = selected;
        // Auto-guess companion SPR if in same folder
        if (!sprPath) {
          const guessedSpr = selected.replace(/\.dat$/i, ".spr");
          sprPath = guessedSpr;
        }
        // Auto-set default output dir
        if (!outputDir) {
          const parentDir = selected.substring(
            0,
            Math.max(selected.lastIndexOf("/"), selected.lastIndexOf("\\")),
          );
          outputDir = parentDir + "/converted_assets";
        }
        await autoInspect();
      }
    } catch (error) {
      console.error("Error browsing DAT file:", error);
    }
  }

  async function browseSprFile() {
    try {
      const selected = await open({
        multiple: false,
        filters: [{ name: "Tibia SPR", extensions: ["spr"] }],
      });
      if (typeof selected === "string" && selected) {
        sprPath = selected;
        await autoInspect();
      }
    } catch (error) {
      console.error("Error browsing SPR file:", error);
    }
  }

  async function browseOutputDir() {
    try {
      const selected = await open({ directory: true, multiple: false });
      if (typeof selected === "string" && selected) {
        outputDir = selected;
      }
    } catch (error) {
      console.error("Error browsing output dir:", error);
    }
  }

  // ── Auto Inspection ────────────────────────────────────────────────────────

  async function autoInspect() {
    if (!datPath || !sprPath) return;

    isDetecting = true;
    errorMessage = null;
    try {
      const info = await invoke<LegacyDetectedInfo>(
        COMMANDS.DETECT_LEGACY_FILES,
        {
          datPath,
          sprPath,
        },
      );
      detectedInfo = info;

      if (selectedVersionId === 0) {
        extendedSprites = info.is_extended;
        transparency = info.suggested_transparency;
        frameGroups = info.suggested_frame_groups;
        improvedAnimations = info.suggested_improved_animations;
      }
    } catch (e: any) {
      console.warn("Could not inspect legacy files:", e);
    } finally {
      isDetecting = false;
    }
  }

  function handleVersionChange(e: Event) {
    const target = e.target as HTMLSelectElement;
    const vId = parseInt(target.value, 10);
    selectedVersionId = vId;

    if (vId > 0) {
      const ver = versions.find((v) => v.id === vId);
      if (ver) {
        extendedSprites = ver.default_extended;
        frameGroups = ver.default_frame_groups;
        improvedAnimations = ver.default_improved_animations;
      }
    } else if (detectedInfo) {
      extendedSprites = detectedInfo.is_extended;
      frameGroups = detectedInfo.suggested_frame_groups;
      improvedAnimations = detectedInfo.suggested_improved_animations;
    }
  }

  // ── Conversion Execution ───────────────────────────────────────────────────

  async function startConversion() {
    if (!datPath) {
      showStatus("Por favor, selecione o arquivo .dat", "error");
      return;
    }
    if (!sprPath) {
      showStatus("Por favor, selecione o arquivo .spr", "error");
      return;
    }
    if (!outputDir) {
      showStatus("Por favor, selecione a pasta de destino dos assets", "error");
      return;
    }

    isConverting = true;
    conversionResult = null;
    errorMessage = null;
    conversionProgressText = "Lendo e decodificando sprites legadas...";

    try {
      const result = await invoke<ConversionResult>(
        COMMANDS.CONVERT_LEGACY_TO_ASSETS,
        {
          options: {
            dat_path: datPath,
            spr_path: sprPath,
            output_dir: outputDir,
            version_id: selectedVersionId > 0 ? selectedVersionId : null,
            extended_sprites: extendedSprites,
            transparency: transparency,
            frame_groups: frameGroups,
            improved_animations: improvedAnimations,
            export_aec: exportAec,
            project_name: projectName || "converted_legacy_assets",
          },
        },
      );

      conversionResult = result;
      showStatus("Conversão concluída com sucesso!", "success");

      if (openAfterConvert) {
        await loadConvertedAssets(result.output_dir);
      }
    } catch (e: any) {
      console.error("Conversion failed:", e);
      errorMessage = typeof e === "string" ? e : e?.message || String(e);
      showStatus("Erro durante a conversão: " + errorMessage, "error");
    } finally {
      isConverting = false;
      conversionProgressText = "";
    }
  }

  async function loadConvertedAssets(path: string) {
    try {
      await invoke(COMMANDS.SET_TIBIA_BASE_PATH, { tibiaPath: path });
      appState.tibiaPath = path;
      const result = await loadAppearancesForAssetsEditor(path);
      await loadSpecialMeaningIds();
      assetsState.currentStats = result;
      assetsState.viewMode = "categories";
      appState.currentView = "assets-editor";
      showStatus("Assets convertidos carregados no editor!", "success");
    } catch (err: any) {
      console.error("Failed to auto-load converted assets:", err);
      showStatus("Não foi possível carregar os assets no editor: " + err, "error");
    }
  }

  function goBack() {
    if (appState.tibiaPath) {
      assetsState.viewMode = "categories";
    } else {
      appState.currentView = "launcher";
    }
  }
</script>

<div class="converter-page">
  <!-- Top Navigation Bar -->
  <header class="converter-header">
    <div class="header-left">
      <button class="back-btn" onclick={goBack} title="Voltar">
        ← Voltar
      </button>
      <div class="title-group">
        <h1>🔄 Conversor de .DAT & .SPR para Assets</h1>
        <p class="subtitle">
          Converta clientes legados do Tibia (7.10 até 10.99+ do ObjectBuilder) para o formato moderno de Assets (Protobuf appearances.dat, LZMA sheets & pacotes .AEC)
        </p>
      </div>
    </div>
  </header>

  <div class="converter-content">
    <!-- Left Column: Source & Options -->
    <div class="column-panel main-panel">
      <!-- 1. Source Files Card -->
      <section class="config-card">
        <div class="card-header">
          <span class="card-step">1</span>
          <h2>Arquivos de Origem Legados</h2>
        </div>
        <div class="form-grid">
          <!-- DAT File -->
          <div class="form-group">
            <label for="dat-path">Arquivo .DAT</label>
            <div class="input-with-button">
              <input
                id="dat-path"
                type="text"
                bind:value={datPath}
                placeholder="Selecione o arquivo .dat..."
                readonly
              />
              <button class="browse-btn" onclick={browseDatFile}>Procurar...</button>
            </div>
          </div>

          <!-- SPR File -->
          <div class="form-group">
            <label for="spr-path">Arquivo .SPR</label>
            <div class="input-with-button">
              <input
                id="spr-path"
                type="text"
                bind:value={sprPath}
                placeholder="Selecione o arquivo .spr..."
                readonly
              />
              <button class="browse-btn" onclick={browseSprFile}>Procurar...</button>
            </div>
          </div>

          <!-- Output Directory -->
          <div class="form-group full-width">
            <label for="output-dir">Pasta de Destino dos Assets Modernos</label>
            <div class="input-with-button">
              <input
                id="output-dir"
                type="text"
                bind:value={outputDir}
                placeholder="Diretório onde os assets convertidos serão salvos..."
              />
              <button class="browse-btn" onclick={browseOutputDir}>Procurar...</button>
            </div>
          </div>
        </div>
      </section>

      <!-- 2. Version & Features Settings -->
      <section class="config-card">
        <div class="card-header">
          <span class="card-step">2</span>
          <h2>Versão e Recursos do Cliente</h2>
        </div>
        
        <div class="form-grid">
          <!-- Version Selector -->
          <div class="form-group full-width">
            <label for="version-select">Versão do Formato (.DAT / .SPR)</label>
            <select id="version-select" value={selectedVersionId} onchange={handleVersionChange}>
              <option value={0}>
                ✨ Auto-detectar por assinatura
                {#if detectedInfo}
                  ({detectedInfo.detected_version_name})
                {/if}
              </option>
              {#each versions as ver}
                <option value={ver.id}>
                  {ver.name} (Struct {ver.structure}, DAT: 0x{ver.dat_signature.toString(16).toUpperCase()})
                </option>
              {/each}
            </select>
          </div>
        </div>

        <!-- Toggles Grid -->
        <div class="toggles-grid">
          <label class="checkbox-card" class:active={extendedSprites}>
            <input type="checkbox" bind:checked={extendedSprites} />
            <div class="checkbox-info">
              <span class="checkbox-title">Extended Sprites (u32)</span>
              <span class="checkbox-desc">Suporte a clientes com mais de 65.535 sprites (Tibia 9.60+ ou OTC).</span>
            </div>
          </label>

          <label class="checkbox-card" class:active={transparency}>
            <input type="checkbox" bind:checked={transparency} />
            <div class="checkbox-info">
              <span class="checkbox-title">Transparência RGBA (OTClient)</span>
              <span class="checkbox-desc">Sprites com 4 canais de cor e canal alpha real de 32-bit.</span>
            </div>
          </label>

          <label class="checkbox-card" class:active={frameGroups}>
            <input type="checkbox" bind:checked={frameGroups} />
            <div class="checkbox-info">
              <span class="checkbox-title">Frame Groups (Outfits)</span>
              <span class="checkbox-desc">Outfits divididos em animação parada (Idle) e andando (Moving).</span>
            </div>
          </label>

          <label class="checkbox-card" class:active={improvedAnimations}>
            <input type="checkbox" bind:checked={improvedAnimations} />
            <div class="checkbox-info">
              <span class="checkbox-title">Animações Avançadas</span>
              <span class="checkbox-desc">Durações personalizadas por frame (min/max ms e loop count).</span>
            </div>
          </label>

          <label class="checkbox-card" class:active={exportAec}>
            <input type="checkbox" bind:checked={exportAec} />
            <div class="checkbox-info">
              <span class="checkbox-title">Exportar Pacote .AEC</span>
              <span class="checkbox-desc">Gera bundle .aec + companion com sprites para importação direta.</span>
            </div>
          </label>

          <label class="checkbox-card" class:active={openAfterConvert}>
            <input type="checkbox" bind:checked={openAfterConvert} />
            <div class="checkbox-info">
              <span class="checkbox-title">Abrir no Editor ao Concluir</span>
              <span class="checkbox-desc">Carrega os novos assets imediatamente no Canary Studio Editor.</span>
            </div>
          </label>
        </div>
      </section>

      <!-- Action Button -->
      <div class="action-footer">
        <button
          class="start-convert-btn"
          disabled={isConverting || !datPath || !sprPath || !outputDir}
          onclick={startConversion}
        >
          {#if isConverting}
            <span class="spinner"></span>
            Convertendo Assets...
          {:else}
            🚀 Iniciar Conversão para Assets
          {/if}
        </button>
      </div>
    </div>

    <!-- Right Column: Inspection & Result -->
    <div class="column-panel side-panel">
      <!-- Inspection Card -->
      <section class="info-card">
        <h3>📊 Inspeção dos Arquivos Legados</h3>

        {#if isDetecting}
          <div class="inspect-loading">
            <span class="spinner"></span> Inspecionando cabeçalhos...
          </div>
        {:else if detectedInfo}
          <div class="inspect-stats">
            <div class="stat-badge version-badge">
              <span class="stat-label">Versão Detectada</span>
              <span class="stat-val highlight">{detectedInfo.detected_version_name}</span>
            </div>

            <div class="stat-row">
              <span class="stat-label">Assinatura DAT:</span>
              <span class="stat-val code">0x{detectedInfo.dat_signature.toString(16).toUpperCase()}</span>
            </div>
            <div class="stat-row">
              <span class="stat-label">Assinatura SPR:</span>
              <span class="stat-val code">0x{detectedInfo.spr_signature.toString(16).toUpperCase()}</span>
            </div>

            <hr class="stat-divider" />

            <div class="stats-mini-grid">
              <div class="mini-stat">
                <span class="mini-label">📦 Itens (Objects)</span>
                <span class="mini-val">{detectedInfo.object_count - 99}</span>
              </div>
              <div class="mini-stat">
                <span class="mini-label">👕 Outfits / Criaturas</span>
                <span class="mini-val">{detectedInfo.outfit_count}</span>
              </div>
              <div class="mini-stat">
                <span class="mini-label">✨ Efeitos</span>
                <span class="mini-val">{detectedInfo.effect_count}</span>
              </div>
              <div class="mini-stat">
                <span class="mini-label">🏹 Mísseis</span>
                <span class="mini-val">{detectedInfo.missile_count}</span>
              </div>
            </div>

            <hr class="stat-divider" />

            <div class="stat-row total-row">
              <span class="stat-label">Total de Things:</span>
              <span class="stat-val">{detectedInfo.total_things}</span>
            </div>
            <div class="stat-row total-row">
              <span class="stat-label">Total de Sprites:</span>
              <span class="stat-val highlight">{detectedInfo.sprite_count.toLocaleString()}</span>
            </div>
          </div>
        {:else}
          <div class="empty-hint">
            Selecione o arquivo .DAT e .SPR para inspecionar os detalhes e contagens de Things.
          </div>
        {/if}
      </section>

      <!-- Conversion Feedback / Results -->
      {#if isConverting}
        <section class="progress-card">
          <span class="spinner large"></span>
          <h4>Processando Conversão</h4>
          <p>{conversionProgressText || "Compactando sprites e gerando appearances.dat..."}</p>
        </section>
      {:else if conversionResult}
        <section class="result-card">
          <div class="result-header">
            <span class="result-icon">✅</span>
            <h4>Conversão Realizada!</h4>
          </div>
          <div class="result-body">
            <p><strong>Tempo decorrido:</strong> {(conversionResult.elapsed_ms / 1000).toFixed(2)}s</p>
            <p><strong>Sprites compiladas:</strong> {conversionResult.sprites_converted.toLocaleString()} em {conversionResult.sheets_created} folhas LZMA (.cwm)</p>
            <p><strong>Appearances salvas:</strong> {conversionResult.appearances_path}</p>
            {#if conversionResult.aec_path}
              <p><strong>Pacote .AEC gerado:</strong> {conversionResult.aec_path}</p>
            {/if}
          </div>
          <button
            class="open-editor-btn"
            onclick={() => loadConvertedAssets(conversionResult!.output_dir)}
          >
            📂 Abrir no Editor Agora
          </button>
        </section>
      {:else if errorMessage}
        <section class="error-card">
          <h4>❌ Falha na Conversão</h4>
          <p>{errorMessage}</p>
        </section>
      {/if}
    </div>
  </div>
</div>

<style>
  .converter-page {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: var(--bg-primary, #12141a);
    color: var(--text-primary, #e2e8f0);
    overflow-y: auto;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Oxygen, Ubuntu, Cantarell, sans-serif;
  }

  .converter-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1rem 2rem;
    background: var(--bg-secondary, #1a1d26);
    border-bottom: 1px solid var(--border-color, #2d3345);
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 1.5rem;
  }

  .back-btn {
    background: var(--bg-tertiary, #262b3a);
    border: 1px solid var(--border-color, #384259);
    color: var(--text-primary, #e2e8f0);
    padding: 0.5rem 1rem;
    border-radius: 6px;
    cursor: pointer;
    font-weight: 500;
    transition: all 0.2s;
  }

  .back-btn:hover {
    background: var(--accent-color, #3b82f6);
    border-color: var(--accent-color, #3b82f6);
    color: white;
  }

  .title-group h1 {
    margin: 0;
    font-size: 1.35rem;
    font-weight: 700;
  }

  .title-group .subtitle {
    margin: 0.25rem 0 0 0;
    font-size: 0.85rem;
    color: var(--text-muted, #94a3b8);
  }

  .converter-content {
    display: grid;
    grid-template-columns: 1.6fr 1fr;
    gap: 1.5rem;
    padding: 2rem;
    max-width: 1500px;
    margin: 0 auto;
    width: 100%;
    box-sizing: border-box;
  }

  .column-panel {
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
  }

  .config-card, .info-card, .result-card, .error-card, .progress-card {
    background: var(--bg-secondary, #1a1d26);
    border: 1px solid var(--border-color, #2d3345);
    border-radius: 10px;
    padding: 1.5rem;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.25);
  }

  .card-header {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    margin-bottom: 1.25rem;
  }

  .card-step {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    background: var(--accent-color, #3b82f6);
    color: white;
    font-weight: bold;
    border-radius: 50%;
    font-size: 0.85rem;
  }

  .card-header h2 {
    margin: 0;
    font-size: 1.15rem;
    font-weight: 600;
  }

  .form-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1rem;
  }

  .form-group {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }

  .form-group.full-width {
    grid-column: 1 / -1;
  }

  .form-group label {
    font-size: 0.85rem;
    font-weight: 500;
    color: var(--text-secondary, #cbd5e1);
  }

  .input-with-button {
    display: flex;
    gap: 0.5rem;
  }

  .input-with-button input, select {
    flex: 1;
    background: var(--bg-input, #0f1218);
    border: 1px solid var(--border-color, #2d3345);
    color: var(--text-primary, #f1f5f9);
    padding: 0.6rem 0.8rem;
    border-radius: 6px;
    font-size: 0.9rem;
    outline: none;
    transition: border-color 0.2s;
  }

  .input-with-button input:focus, select:focus {
    border-color: var(--accent-color, #3b82f6);
  }

  .browse-btn {
    background: var(--bg-tertiary, #262b3a);
    border: 1px solid var(--border-color, #384259);
    color: var(--text-primary, #e2e8f0);
    padding: 0.6rem 1rem;
    border-radius: 6px;
    cursor: pointer;
    font-weight: 500;
    white-space: nowrap;
    transition: all 0.2s;
  }

  .browse-btn:hover {
    background: #333c52;
  }

  .toggles-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.85rem;
    margin-top: 1.25rem;
  }

  .checkbox-card {
    display: flex;
    align-items: flex-start;
    gap: 0.75rem;
    background: var(--bg-input, #0f1218);
    border: 1px solid var(--border-color, #2d3345);
    border-radius: 8px;
    padding: 0.85rem;
    cursor: pointer;
    transition: all 0.2s;
  }

  .checkbox-card:hover {
    border-color: #475569;
  }

  .checkbox-card.active {
    border-color: var(--accent-color, #3b82f6);
    background: rgba(59, 130, 246, 0.08);
  }

  .checkbox-card input[type="checkbox"] {
    margin-top: 0.2rem;
    cursor: pointer;
  }

  .checkbox-info {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }

  .checkbox-title {
    font-size: 0.9rem;
    font-weight: 600;
  }

  .checkbox-desc {
    font-size: 0.75rem;
    color: var(--text-muted, #94a3b8);
    line-height: 1.3;
  }

  .action-footer {
    display: flex;
    justify-content: flex-end;
  }

  .start-convert-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.75rem;
    background: linear-gradient(135deg, #2563eb, #1d4ed8);
    color: white;
    border: none;
    padding: 1rem 2rem;
    font-size: 1.05rem;
    font-weight: 600;
    border-radius: 8px;
    cursor: pointer;
    width: 100%;
    box-shadow: 0 4px 14px rgba(37, 99, 235, 0.4);
    transition: all 0.2s;
  }

  .start-convert-btn:hover:not(:disabled) {
    background: linear-gradient(135deg, #1d4ed8, #1e40af);
    transform: translateY(-1px);
    box-shadow: 0 6px 18px rgba(37, 99, 235, 0.5);
  }

  .start-convert-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
    transform: none;
    box-shadow: none;
  }

  .info-card h3 {
    margin: 0 0 1rem 0;
    font-size: 1.1rem;
    font-weight: 600;
  }

  .inspect-stats {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }

  .version-badge {
    background: rgba(59, 130, 246, 0.15);
    border: 1px solid rgba(59, 130, 246, 0.3);
    border-radius: 6px;
    padding: 0.6rem 0.8rem;
    margin-bottom: 0.5rem;
  }

  .stat-row {
    display: flex;
    justify-content: space-between;
    font-size: 0.88rem;
  }

  .stat-label {
    color: var(--text-secondary, #94a3b8);
  }

  .stat-val {
    font-weight: 600;
  }

  .stat-val.code {
    font-family: monospace;
    color: #38bdf8;
  }

  .stat-val.highlight {
    color: #4ade80;
  }

  .stat-divider {
    border: none;
    border-top: 1px solid var(--border-color, #2d3345);
    margin: 0.5rem 0;
  }

  .stats-mini-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.6rem;
  }

  .mini-stat {
    display: flex;
    flex-direction: column;
    background: var(--bg-input, #0f1218);
    border: 1px solid var(--border-color, #2d3345);
    padding: 0.5rem 0.75rem;
    border-radius: 6px;
  }

  .mini-label {
    font-size: 0.75rem;
    color: var(--text-muted, #94a3b8);
  }

  .mini-val {
    font-size: 1.1rem;
    font-weight: 700;
    color: #f8fafc;
    margin-top: 0.2rem;
  }

  .total-row {
    font-size: 0.95rem;
  }

  .empty-hint {
    padding: 1.5rem;
    text-align: center;
    color: var(--text-muted, #64748b);
    font-size: 0.85rem;
    line-height: 1.4;
  }

  .progress-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: 0.75rem;
    padding: 2rem;
  }

  .result-card {
    border-color: #22c55e;
    background: rgba(34, 197, 94, 0.05);
  }

  .result-header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 0.75rem;
  }

  .result-header h4 {
    margin: 0;
    font-size: 1.1rem;
    color: #4ade80;
  }

  .result-body p {
    margin: 0.4rem 0;
    font-size: 0.85rem;
    word-break: break-all;
  }

  .open-editor-btn {
    margin-top: 1rem;
    width: 100%;
    background: #16a34a;
    color: white;
    border: none;
    padding: 0.75rem;
    border-radius: 6px;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.2s;
  }

  .open-editor-btn:hover {
    background: #15803d;
  }

  .error-card {
    border-color: #ef4444;
    background: rgba(239, 68, 68, 0.05);
  }

  .error-card h4 {
    margin: 0 0 0.5rem 0;
    color: #f87171;
  }

  .error-card p {
    margin: 0;
    font-size: 0.85rem;
    color: #fca5a5;
  }

  .spinner {
    width: 18px;
    height: 18px;
    border: 2px solid rgba(255, 255, 255, 0.3);
    border-top-color: white;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
    display: inline-block;
  }

  .spinner.large {
    width: 32px;
    height: 32px;
    border-width: 3px;
    border-top-color: #3b82f6;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }
</style>
