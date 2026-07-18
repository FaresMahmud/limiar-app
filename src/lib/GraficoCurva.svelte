<script lang="ts">
  import { onMount } from 'svelte';

  interface StatsDto {
    grupo_id: number;
    grupo_nome: string;
    grupo_cor: string;
    timepoint_id: number;
    timepoint_rotulo: string;
    timepoint_ordem: number;
    n: number;
    media_geometrica_g: number;
    limite_inferior_g: number | null;
    limite_superior_g: number | null;
  }

  interface Timepoint {
    id: number;
    rotulo: string;
    ordem: number;
  }

  // Props
  let { stats = [], timepoints = [] }: { stats: StatsDto[], timepoints: Timepoint[] } = $props();

  // Dimensões do SVG
  const width = 700;
  const height = 400;
  const padding = { top: 40, right: 150, bottom: 50, left: 60 };

  // Agrupar timepoints ordenados
  const tpsOrdenados = $derived(
    [...timepoints].sort((a, b) => a.ordem - b.ordem)
  );

  // Mapear posição X para cada ID de timepoint
  const xMap = $derived.by(() => {
    const map = new Map<number, number>();
    const count = tpsOrdenados.length;
    const chartWidth = width - padding.left - padding.right;
    
    tpsOrdenados.forEach((tp, index) => {
      const x = count > 1 
        ? padding.left + (index / (count - 1)) * chartWidth
        : padding.left + chartWidth / 2;
      map.set(tp.id, x);
    });
    return map;
  });

  // Achar o valor máximo Y (considerando limites superiores)
  const maxY = $derived.by(() => {
    let max = 0.5; // mínimo padrão
    stats.forEach(s => {
      const val = s.limite_superior_g ?? s.media_geometrica_g;
      if (val > max) max = val;
    });
    return max * 1.15; // 15% de margem no topo
  });

  // Ticks para o eixo Y
  const yTicks = $derived.by(() => {
    const ticks = [];
    const step = maxY / 5;
    for (let i = 0; i <= 5; i++) {
      ticks.push(i * step);
    }
    return ticks;
  });

  // Função para obter a coordenada Y de um valor em gramas
  function getY(valor: number): number {
    const chartHeight = height - padding.top - padding.bottom;
    return padding.top + chartHeight - (valor / maxY) * chartHeight;
  }

  // Agrupar estatísticas por grupo
  interface GrupoLinha {
    id: number;
    nome: string;
    cor: string;
    pontos: StatsDto[];
  }

  const gruposLinhas = $derived.by(() => {
    const map = new Map<number, GrupoLinha>();
    stats.forEach(s => {
      if (!map.has(s.grupo_id)) {
        map.set(s.grupo_id, {
          id: s.grupo_id,
          nome: s.grupo_nome,
          cor: s.grupo_cor,
          pontos: []
        });
      }
      map.get(s.grupo_id)!.pontos.push(s);
    });

    // Ordenar os pontos de cada grupo pela ordem do timepoint
    const linhas = Array.from(map.values());
    linhas.forEach(l => {
      l.pontos.sort((a, b) => a.timepoint_ordem - b.timepoint_ordem);
    });
    return linhas;
  });

  // Estado para controle de tooltip de hover
  let activeTooltip = $state<{
    x: number;
    y: number;
    s: StatsDto;
  } | null>(null);

  let svgElement: SVGSVGElement | null = $state(null);

  // Exportar SVG como arquivo
  function exportarSVG() {
    if (!svgElement) return;
    try {
      const serializer = new XMLSerializer();
      let source = serializer.serializeToString(svgElement);
      
      // Adicionar namespaces corretos e estilos embutidos
      if (!source.match(/^<svg[^>]+xmlns="http:\/\/www\.w3\.org\/2000\/svg"/)) {
        source = source.replace(/^<svg/, '<svg xmlns="http://www.w3.org/2000/svg"');
      }
      
      const blob = new Blob([source], { type: 'image/svg+xml;charset=utf-8' });
      const url = URL.createObjectURL(blob);
      const link = document.createElement('a');
      link.href = url;
      link.download = 'curva_temporal.svg';
      link.click();
      URL.revokeObjectURL(url);
    } catch (e) {
      console.error('Falha ao exportar SVG:', e);
      alert('Erro ao exportar gráfico em SVG.');
    }
  }

  // Exportar PNG via Canvas
  function exportarPNG() {
    if (!svgElement) return;
    try {
      const serializer = new XMLSerializer();
      let source = serializer.serializeToString(svgElement);
      if (!source.match(/^<svg[^>]+xmlns="http:\/\/www\.w3\.org\/2000\/svg"/)) {
        source = source.replace(/^<svg/, '<svg xmlns="http://www.w3.org/2000/svg"');
      }

      const img = new Image();
      img.src = 'data:image/svg+xml;charset=utf-8,' + encodeURIComponent(source);
      
      img.onload = () => {
        const canvas = document.createElement('canvas');
        // Usar fator de escala 2 para alta definição (Retina/High-DPI)
        const scale = 2;
        canvas.width = width * scale;
        canvas.height = height * scale;
        
        const ctx = canvas.getContext('2d');
        if (!ctx) return;
        
        ctx.scale(scale, scale);
        // Preencher fundo com branco
        ctx.fillStyle = '#ffffff';
        ctx.fillRect(0, 0, width, height);
        
        ctx.drawImage(img, 0, 0);
        
        const url = canvas.toDataURL('image/png');
        const link = document.createElement('a');
        link.href = url;
        link.download = 'curva_temporal.png';
        link.click();
      };
    } catch (e) {
      console.error('Falha ao exportar PNG:', e);
      alert('Erro ao exportar gráfico em PNG.');
    }
  }
</script>

<div class="chart-wrapper">
  <div class="chart-actions">
    <button class="btn-export-img" onclick={exportarSVG}>📥 Baixar SVG</button>
    <button class="btn-export-img" onclick={exportarPNG}>🖼️ Baixar PNG</button>
  </div>

  <div class="svg-container">
    <svg 
      bind:this={svgElement}
      {width} 
      {height} 
      viewBox="0 0 {width} {height}"
      class="temporal-chart-svg"
      style="background-color: var(--bg); font-family: system-ui, sans-serif;"
    >
      <!-- Definições para sombras ou gradientes se necessário -->
      <defs>
        <style>
          .grid-line { stroke: var(--border); stroke-dasharray: 4 4; opacity: 0.5; }
          .axis-line { stroke: var(--text); stroke-width: 1.5; opacity: 0.8; }
          .axis-text { fill: var(--text); font-size: 11px; }
          .legend-text { fill: var(--text-h); font-size: 12px; font-weight: 500; }
        </style>
      </defs>

      <!-- Linhas de Grid Horizontal -->
      {#each yTicks as tick}
        {@const y = getY(tick)}
        <line x1={padding.left} y1={y} x2={width - padding.right} y2={y} class="grid-line" />
        <text x={padding.left - 8} y={y + 4} text-anchor="end" class="axis-text">{tick.toFixed(2)}g</text>
      {/each}

      <!-- Eixo X Labels (Timepoints) -->
      {#each tpsOrdenados as tp}
        {@const x = xMap.get(tp.id) ?? padding.left}
        <text x={x} y={height - padding.bottom + 20} text-anchor="middle" class="axis-text">{tp.rotulo}</text>
        <line x1={x} y1={height - padding.bottom} x2={x} y2={height - padding.bottom + 5} class="axis-line" style="stroke-width: 1;" />
      {/each}

      <!-- Linha dos Eixos -->
      <!-- Eixo Y -->
      <line x1={padding.left} y1={padding.top} x2={padding.left} y2={height - padding.bottom} class="axis-line" />
      <!-- Eixo X -->
      <line x1={padding.left} y1={height - padding.bottom} x2={width - padding.right} y2={height - padding.bottom} class="axis-line" />

      <!-- Renderização das Curvas e Barras de Erro por Grupo -->
      {#each gruposLinhas as linha}
        <!-- Desenha as linhas que conectam os pontos -->
        {#if linha.pontos.length > 1}
          {@const pathPoints = linha.pontos
            .map(p => {
              const x = xMap.get(p.timepoint_id);
              const y = getY(p.media_geometrica_g);
              return x !== undefined ? `${x},${y}` : null;
            })
            .filter(p => p !== null)
            .join(' ')}
          <polyline 
            points={pathPoints} 
            fill="none" 
            stroke={linha.cor} 
            stroke-width="2.5" 
            stroke-linecap="round"
            stroke-linejoin="round"
          />
        {/if}

        <!-- Desenha Barras de Erro (Asimétricas) e Círculos de Média -->
        {#each linha.pontos as p}
          {@const x = xMap.get(p.timepoint_id)}
          {#if x !== undefined}
            {@const yMean = getY(p.media_geometrica_g)}
            
            <!-- Barra de erro vertical -->
            {#if p.limite_inferior_g !== null && p.limite_superior_g !== null}
              {@const yMin = getY(p.limite_inferior_g)}
              {@const yMax = getY(p.limite_superior_g)}
              
              <!-- Haste central da barra de erro -->
              <line x1={x} y1={yMin} x2={x} y2={yMax} stroke={linha.cor} stroke-width="1.5" />
              <!-- Cap horizontal inferior -->
              <line x1={x - 4} y1={yMin} x2={x + 4} y2={yMin} stroke={linha.cor} stroke-width="1.5" />
              <!-- Cap horizontal superior -->
              <line x1={x - 4} y1={yMax} x2={x + 4} y2={yMax} stroke={linha.cor} stroke-width="1.5" />
            {/if}

            <!-- Marcador central (Média Geométrica) -->
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <circle 
              cx={x} 
              cy={yMean} 
              r="5" 
              fill={linha.cor}
              stroke="#ffffff"
              stroke-width="1.5"
              style="cursor: pointer;"
              onmouseenter={(e) => {
                const rect = e.currentTarget.getBoundingClientRect();
                activeTooltip = {
                  x: x,
                  y: yMean - 10,
                  s: p
                };
              }}
              onmouseleave={() => {
                activeTooltip = null;
              }}
            />
          {/if}
        {/each}
      {/each}

      <!-- Legenda Lateral Direita -->
      {#each gruposLinhas as linha, index}
        {@const yPos = padding.top + index * 24}
        <rect x={width - padding.right + 15} y={yPos} width="12" height="12" fill={linha.cor} rx="2" />
        <text x={width - padding.right + 34} y={yPos + 10} class="legend-text">{linha.nome}</text>
      {/each}

      <!-- Tooltip SVG Flutuante -->
      {#if activeTooltip}
        {@const boxW = 160}
        {@const boxH = 90}
        <!-- Garantir que o tooltip não corte nas bordas -->
        {@const toolX = Math.max(10, Math.min(width - boxW - 10, activeTooltip.x - boxW / 2))}
        {@const toolY = Math.max(10, activeTooltip.y - boxH - 5)}
        
        <g style="pointer-events: none;">
          <!-- Sombra do balão -->
          <rect x={toolX + 2} y={toolY + 2} width={boxW} height={boxH} fill="#000000" opacity="0.1" rx="6" />
          <!-- Corpo do balão -->
          <rect x={toolX} y={toolY} width={boxW} height={boxH} fill="var(--code-bg)" stroke="var(--border)" stroke-width="1.5" rx="6" />
          
          <!-- Título (Grupo) -->
          <rect x={toolX + 8} y={toolY + 8} width="8" height="8" fill={activeTooltip.s.grupo_cor} rx="2" />
          <text x={toolX + 20} y={toolY + 16} font-size="11px" font-weight="bold" fill="var(--text-h)">{activeTooltip.s.grupo_nome}</text>
          
          <!-- Timepoint -->
          <text x={toolX + 8} y={toolY + 32} font-size="10px" fill="var(--text)" opacity="0.7">Timepoint: {activeTooltip.s.timepoint_rotulo}</text>
          
          <!-- Média geométrica -->
          <text x={toolX + 8} y={toolY + 48} font-size="11px" font-weight="600" fill="var(--accent)">Média: {activeTooltip.s.media_geometrica_g.toFixed(3)}g</text>
          
          <!-- Margens de Erro -->
          <text x={toolX + 8} y={toolY + 62} font-size="9px" fill="var(--text)">
            {#if activeTooltip.s.limite_inferior_g !== null}
              EP: [{activeTooltip.s.limite_inferior_g.toFixed(3)}g - {activeTooltip.s.limite_superior_g?.toFixed(3)}g]
            {:else}
              EP: Indefinido (N=1)
            {/if}
          </text>
          
          <!-- N de animais -->
          <text x={toolX + 8} y={toolY + 76} font-size="9px" fill="var(--text)" opacity="0.8">Amostra: N = {activeTooltip.s.n}</text>
        </g>
      {/if}
    </svg>
  </div>
</div>

<style>
  .chart-wrapper {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    background-color: var(--code-bg);
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 16px;
    margin-top: 16px;
  }
  
  .chart-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-bottom: 12px;
  }
  
  .btn-export-img {
    background-color: var(--bg);
    border: 1px solid var(--border);
    color: var(--text-h);
    padding: 6px 12px;
    border-radius: 6px;
    font-size: 12px;
    cursor: pointer;
    font-weight: 500;
    transition: all 0.2s;
  }
  
  .btn-export-img:hover {
    background-color: var(--border);
    color: var(--accent);
  }

  .svg-container {
    display: flex;
    justify-content: center;
    overflow-x: auto;
    background: var(--bg);
    border-radius: 8px;
    border: 1px solid var(--border);
    padding: 8px;
  }

  .temporal-chart-svg {
    max-width: 100%;
    height: auto;
  }
</style>
