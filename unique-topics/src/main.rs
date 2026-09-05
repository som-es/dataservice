use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::path::PathBuf;

use clap::Parser;
use combx::connect_pg;
use eframe::egui;
use egui::{Color32, Pos2, Vec2};
use egui_graphs::{
    DefaultEdgeShape, DefaultNodeShape, DisplayEdge, DisplayNode, Graph, GraphView, Layout,
    LayoutState, SettingsInteraction, SettingsNavigation, SettingsStyle,
};
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use fasttext::FastText;
use petgraph::Undirected;
use petgraph::stable_graph::StableGraph;

use crate::unique_topics::{TopicSource, all_used_topics, hallucinated_ai_topics_by_model};

mod unique_topics;

const MODEL_PATH: &str = "unique-topics/models/cc.de.300.bin";
const FALLBACK_MODEL_PATH: &str = "models/cc.de.300.bin";
const FASTEMBED_CACHE_DIR: &str = ".fastembed_cache";
const GRAPH_VIEW_ID: &str = "unique-topics-static-similarity-layout-v4";
const USED_CLUSTER_SPACING: f32 = 420.0;
const USED_JITTER: f32 = 10.0;
const CLUSTER_MIN_RADIUS: f32 = 20.0;
const CLUSTER_MAX_RADIUS: f32 = 115.0;
const CLUSTER_RING_SPACING: f32 = 6.0;
const TOP_USED_EDGES: usize = 2;
const MIN_EDGE_SIMILARITY: f32 = 0.36;
const INTER_CLUSTER_PULL: f32 = 0.05;
const SIMILARITY_RADIUS_EXPONENT: f32 = 2.8;
const SUBCLUSTER_RADIUS_FACTOR: f32 = 0.5;
const SUBCLUSTER_BASE_OFFSET: f32 = 10.0;
const ZOOM_SPEED: f32 = 0.025;

#[derive(Parser, Debug)]
struct Args {
    /// Print Asyl/Asylrecht similarity and exit
    #[arg(long)]
    embedding_check_only: bool,
    /// Use local FastEmbed multilingual-e5-small instead of FastText for graph layout
    #[arg(long)]
    use_fastembed: bool,
    /// Local FastEmbed Hugging Face cache directory
    #[arg(long, default_value = FASTEMBED_CACHE_DIR)]
    fastembed_cache_dir: PathBuf,
}

type TopicGraph = Graph<TopicNode, f32, Undirected>;
type AnchorGroupKey = (usize, Option<usize>);

type TopicGraphView<'a> = GraphView<
    'a,
    TopicNode,
    f32,
    Undirected,
    u32,
    DefaultNodeShape,
    DefaultEdgeShape,
    StaticLayoutState,
    StaticLayout,
>;

#[derive(Clone, Debug)]
struct TopicNode;

#[derive(Clone, Copy, Debug)]
enum TopicKind {
    Used,
    Ai,
}

#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
struct StaticLayoutState;

impl LayoutState for StaticLayoutState {}

#[derive(Debug, Default)]
struct StaticLayout;

impl Layout<StaticLayoutState> for StaticLayout {
    fn from_state(_: StaticLayoutState) -> impl Layout<StaticLayoutState> {
        Self
    }

    fn next<N, E, Ty, Ix, Dn, De>(&mut self, _: &mut Graph<N, E, Ty, Ix, Dn, De>, _: &egui::Ui)
    where
        N: Clone,
        E: Clone,
        Ty: petgraph::EdgeType,
        Ix: petgraph::stable_graph::IndexType,
        Dn: DisplayNode<N, E, Ty, Ix>,
        De: DisplayEdge<N, E, Ty, Ix, Dn>,
    {
    }

    fn state(&self) -> StaticLayoutState {
        StaticLayoutState
    }
}

#[derive(Clone, Debug)]
struct EmbeddedTopic {
    name: String,
    is_used: bool,
    vector: Vec<f32>,
}

#[derive(Clone, Debug)]
struct SimilarityAnchor {
    used_index: usize,
    similarity: f32,
}

#[derive(Clone, Debug)]
struct TopicPlacement {
    topic_index: usize,
    position: Pos2,
    anchors: Vec<SimilarityAnchor>,
}

struct App {
    graph: TopicGraph,
    node_count: usize,
    edge_count: usize,
    used_count: usize,
    ai_count: usize,
    asyl_similarity: f32,
    fastembed_asyl_similarity: Option<f32>,
    layout_label: &'static str,
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(format!(
                    "Topics: {} | Eurovoc: {} | AI-only: {} | edges: {} | layout: {} | FastText Asyl/Asylrecht: {:.3}{}",
                    self.node_count,
                    self.used_count,
                    self.ai_count,
                    self.edge_count,
                    self.layout_label,
                    self.asyl_similarity,
                    self.fastembed_asyl_similarity
                        .map(|similarity| format!(" | embedding model: {similarity:.3}"))
                        .unwrap_or_default()
                ));
            });

            let navigation = SettingsNavigation::new()
                .with_fit_to_screen_enabled(false)
                .with_fit_to_screen_padding(0.08)
                .with_zoom_and_pan_enabled(true)
                .with_zoom_speed(ZOOM_SPEED);
            let interactions = SettingsInteraction::new()
                .with_dragging_enabled(false)
                .with_hover_enabled(true)
                .with_node_selection_enabled(true);
            let style = SettingsStyle::new().with_labels_always(false);

            let mut view: TopicGraphView<'_> = GraphView::new(&mut self.graph)
                .with_interactions(&interactions)
                .with_navigations(&navigation)
                .with_styles(&style)
                .with_id(Some(GRAPH_VIEW_ID.to_owned()));
            ui.add(&mut view);
        });
    }
}

#[tokio::main]
async fn main() -> combx::Result<()> {
    let args = Args::parse();
    let mut fasttext_model = load_model()?;
    if args.embedding_check_only {
        print_embedding_check(&mut fasttext_model, &args);
        return Ok(());
    }

    let pg = connect_pg().await;

    let unique_topics = all_used_topics(&pg, TopicSource::Eurovoc).await?;
    let hallucinated_topics = hallucinated_ai_topics_by_model(&pg, TopicSource::Eurovoc).await?;
    let topic_records = collect_topic_records(unique_topics, hallucinated_topics);
    let (topics, fastembed_asyl_similarity, layout_label) = if args.use_fastembed {
        let cache_dir = fastembed_cache_dir(&args);
        let mut embedding_model = load_fastembed_model(&cache_dir)?;
        let fastembed_asyl_similarity =
            fastembed_embedding_similarity_with_model(&mut embedding_model, "Asyl", "Asylrecht")
                .ok();
        (
            embed_topic_records_with_fastembed(topic_records, &mut embedding_model)?,
            fastembed_asyl_similarity,
            "FastEmbed",
        )
    } else {
        (
            embed_topic_records_with_fasttext(topic_records, &mut fasttext_model),
            None,
            "FastText",
        )
    };

    let asyl_similarity = embedding_similarity(&mut fasttext_model, "Asyl", "Asylrecht");
    println!("FastText sentence-vector similarity Asyl <-> Asylrecht: {asyl_similarity:.4}");
    if let Some(similarity) = fastembed_asyl_similarity {
        println!("FastEmbed multilingual-e5-small similarity Asyl <-> Asylrecht: {similarity:.4}");
    }

    let (graph, used_count, ai_count) = build_graph(&topics);
    let node_count = graph.node_count();
    let edge_count = graph.edge_count();
    println!(
        "Built topic graph with {node_count} nodes ({used_count} Eurovoc, {ai_count} AI-only) and {edge_count} anchor edges"
    );

    let app = App {
        graph,
        node_count,
        edge_count,
        used_count,
        ai_count,
        asyl_similarity,
        fastembed_asyl_similarity,
        layout_label,
    };

    eframe::run_native(
        "Unique topics similarity graph",
        eframe::NativeOptions::default(),
        Box::new(|_| Ok(Box::new(app))),
    )
    .map_err(|err| -> Box<dyn std::error::Error + Send + Sync> { Box::new(err) })?;

    Ok(())
}

fn load_model() -> combx::Result<FastText> {
    FastText::load_model(MODEL_PATH)
        .or_else(|_| FastText::load_model(FALLBACK_MODEL_PATH))
        .map_err(Into::into)
}

fn load_fastembed_model(
    cache_dir: &PathBuf,
) -> Result<TextEmbedding, Box<dyn Error + Send + Sync>> {
    Ok(TextEmbedding::try_new(
        InitOptions::new(EmbeddingModel::MultilingualE5Small)
            .with_cache_dir(cache_dir.clone())
            .with_show_download_progress(false)
            .with_intra_threads(4),
    )?)
}

fn collect_topic_records(
    unique_topics: HashSet<String>,
    hallucinated_topics: Vec<(String, usize, HashSet<String>)>,
) -> Vec<(String, bool)> {
    let mut topics = unique_topics
        .into_iter()
        .map(|topic| (topic, true))
        .collect::<Vec<_>>();

    topics.sort_by(|a, b| a.0.cmp(&b.0));

    let mut seen = topics
        .iter()
        .map(|(topic, _)| topic.clone())
        .collect::<HashSet<_>>();
    let mut ai_topics = hallucinated_topics
        .into_iter()
        .flat_map(|(_, _, topics)| topics)
        .filter(|topic| seen.insert(topic.clone()))
        .map(|topic| (topic, false))
        .collect::<Vec<_>>();

    ai_topics.sort_by(|a, b| a.0.cmp(&b.0));
    topics.extend(ai_topics);
    topics
}

fn embed_topic_records_with_fasttext(
    topics: Vec<(String, bool)>,
    model: &mut FastText,
) -> Vec<EmbeddedTopic> {
    topics
        .into_iter()
        .map(|(name, is_used)| EmbeddedTopic {
            vector: fasttext_topic_embedding(model, &name),
            name,
            is_used,
        })
        .collect()
}

fn embed_topic_records_with_fastembed(
    topics: Vec<(String, bool)>,
    model: &mut TextEmbedding,
) -> Result<Vec<EmbeddedTopic>, Box<dyn Error + Send + Sync>> {
    let inputs = topics
        .iter()
        .map(|(topic, _)| format!("query: {topic}"))
        .collect::<Vec<_>>();
    let vectors = model.embed(inputs, Some(64))?;

    Ok(topics
        .into_iter()
        .zip(vectors)
        .map(|((name, is_used), vector)| EmbeddedTopic {
            name,
            is_used,
            vector,
        })
        .collect())
}

fn build_graph(topics: &[EmbeddedTopic]) -> (TopicGraph, usize, usize) {
    let base: StableGraph<TopicNode, f32, Undirected> = StableGraph::default();
    let mut graph = Graph::from(&base);
    let used_indices = topics
        .iter()
        .enumerate()
        .filter_map(|(index, topic)| topic.is_used.then_some(index))
        .collect::<Vec<_>>();
    let used_count = used_indices.len();
    let ai_count = topics.len().saturating_sub(used_count);
    let used_positions = place_used_nodes(&used_indices);
    let ai_placements = place_ai_nodes(topics, &used_indices, &used_positions);
    let ai_positions = ai_placements
        .iter()
        .map(|placement| (placement.topic_index, placement.position))
        .collect::<HashMap<_, _>>();
    let placements_by_topic = ai_placements
        .into_iter()
        .map(|placement| (placement.topic_index, placement))
        .collect::<HashMap<_, _>>();
    let mut node_ids = HashMap::new();

    for (topic_index, topic) in topics.iter().enumerate() {
        let position = if topic.is_used {
            used_positions[&topic_index]
        } else {
            ai_positions[&topic_index]
        };
        let kind = if topic.is_used {
            TopicKind::Used
        } else {
            TopicKind::Ai
        };
        let node_id =
            graph.add_node_with_label_and_location(TopicNode, topic.name.clone(), position);
        if let Some(node) = graph.node_mut(node_id) {
            match kind {
                TopicKind::Used => node.set_color(Color32::from_rgb(26, 125, 199)),
                TopicKind::Ai => node.set_color(Color32::from_rgb(220, 143, 43)),
            }
        }
        node_ids.insert(topic_index, node_id);
    }

    for placement in placements_by_topic.values() {
        let ai_node = node_ids[&placement.topic_index];
        for anchor in &placement.anchors {
            let used_node = node_ids[&anchor.used_index];
            graph.add_edge_with_label(
                used_node,
                ai_node,
                anchor.similarity,
                format!("{:.2}", anchor.similarity),
            );
        }
    }

    (graph, used_count, ai_count)
}

fn place_used_nodes(used_indices: &[usize]) -> HashMap<usize, Pos2> {
    let mut positions = HashMap::with_capacity(used_indices.len());
    let count = used_indices.len().max(1);
    let columns = (count as f32).sqrt().ceil().max(1.0) as usize;
    let rows = count.div_ceil(columns);
    let x_offset = (columns.saturating_sub(1)) as f32 * USED_CLUSTER_SPACING / 2.0;
    let y_offset = (rows.saturating_sub(1)) as f32 * USED_CLUSTER_SPACING / 2.0;

    for (cursor, &topic_index) in used_indices.iter().enumerate() {
        let col = cursor % columns;
        let row = cursor / columns;
        let angle = golden_angle(cursor);
        let jitter = Vec2::angled(angle) * USED_JITTER;
        positions.insert(
            topic_index,
            Pos2::new(
                col as f32 * USED_CLUSTER_SPACING - x_offset + jitter.x,
                row as f32 * USED_CLUSTER_SPACING - y_offset + jitter.y,
            ),
        );
    }

    positions
}

fn place_ai_nodes(
    topics: &[EmbeddedTopic],
    used_indices: &[usize],
    used_positions: &HashMap<usize, Pos2>,
) -> Vec<TopicPlacement> {
    let mut placements = Vec::new();
    let mut group_counts = HashMap::<AnchorGroupKey, usize>::new();
    let mut group_orders = HashMap::<AnchorGroupKey, usize>::new();
    let mut primary_group_counts = HashMap::<usize, usize>::new();
    let ai_indices = topics
        .iter()
        .enumerate()
        .filter_map(|(index, topic)| (!topic.is_used).then_some(index))
        .collect::<Vec<_>>();

    for (ai_order, topic_index) in ai_indices.into_iter().enumerate() {
        let anchors = nearest_used_anchors(topics, topic_index, used_indices);
        let Some(anchor) = anchors.first() else {
            let angle = golden_angle(ai_order);
            placements.push(TopicPlacement {
                topic_index,
                position: Pos2::new(angle.cos() * 220.0, angle.sin() * 220.0),
                anchors,
            });
            continue;
        };

        let secondary_index = anchors.get(1).map(|anchor| anchor.used_index);
        let group_key = (anchor.used_index, secondary_index);
        let group_order = *group_orders.entry(group_key).or_insert_with(|| {
            let next_order = primary_group_counts.entry(anchor.used_index).or_default();
            let order = *next_order;
            *next_order += 1;
            order
        });
        let group_index = group_counts.entry(group_key).or_default();
        let local_index = *group_index;
        *group_index += 1;

        let primary_center = used_positions[&anchor.used_index];
        let semantic_center = weighted_anchor_center(&anchors, used_positions);
        let normalized_similarity = anchor.similarity.clamp(0.0, 1.0);
        let similarity_radius = CLUSTER_MIN_RADIUS
            + (1.0 - normalized_similarity).powf(SIMILARITY_RADIUS_EXPONENT)
                * (CLUSTER_MAX_RADIUS - CLUSTER_MIN_RADIUS);
        let group_direction =
            anchor_group_direction(primary_center, secondary_index, group_order, used_positions);
        let semantic_offset = (semantic_center - primary_center) * INTER_CLUSTER_PULL;
        let subgroup_offset = group_direction
            * (SUBCLUSTER_BASE_OFFSET + similarity_radius * SUBCLUSTER_RADIUS_FACTOR);
        let center = primary_center + semantic_offset + subgroup_offset;
        let crowding_radius = (local_index as f32).sqrt().floor() * CLUSTER_RING_SPACING;
        let angle = golden_angle(local_index) + group_order as f32 * 0.631;
        let offset = Vec2::angled(angle) * crowding_radius;

        placements.push(TopicPlacement {
            topic_index,
            position: center + offset,
            anchors,
        });
    }

    placements
}

fn anchor_group_direction(
    primary_center: Pos2,
    secondary_index: Option<usize>,
    group_order: usize,
    used_positions: &HashMap<usize, Pos2>,
) -> Vec2 {
    if let Some(secondary_center) =
        secondary_index.and_then(|index| used_positions.get(&index).copied())
    {
        let direction = secondary_center - primary_center;
        let length = (direction.x * direction.x + direction.y * direction.y).sqrt();
        if length > f32::EPSILON {
            return direction / length;
        }
    }

    Vec2::angled(golden_angle(group_order))
}

fn weighted_anchor_center(
    anchors: &[SimilarityAnchor],
    used_positions: &HashMap<usize, Pos2>,
) -> Pos2 {
    let mut weighted_sum = Vec2::ZERO;
    let mut total_weight = 0.0;

    for anchor in anchors {
        let Some(position) = used_positions.get(&anchor.used_index) else {
            continue;
        };
        let weight = (anchor.similarity - MIN_EDGE_SIMILARITY).max(0.0).powf(2.0);
        weighted_sum += position.to_vec2() * weight;
        total_weight += weight;
    }

    if total_weight <= f32::EPSILON {
        anchors
            .first()
            .and_then(|anchor| used_positions.get(&anchor.used_index))
            .copied()
            .unwrap_or(Pos2::ZERO)
    } else {
        (weighted_sum / total_weight).to_pos2()
    }
}

fn nearest_used_anchors(
    topics: &[EmbeddedTopic],
    topic_index: usize,
    used_indices: &[usize],
) -> Vec<SimilarityAnchor> {
    let mut anchors = used_indices
        .iter()
        .map(|&used_index| SimilarityAnchor {
            used_index,
            similarity: cosine_similarity(&topics[topic_index].vector, &topics[used_index].vector),
        })
        .collect::<Vec<_>>();

    anchors.sort_by(|a, b| b.similarity.total_cmp(&a.similarity));
    anchors
        .into_iter()
        .take(TOP_USED_EDGES)
        .filter(|anchor| anchor.similarity >= MIN_EDGE_SIMILARITY)
        .collect()
}

fn embedding_similarity(model: &mut FastText, a: &str, b: &str) -> f32 {
    let a = fasttext_topic_embedding(model, a);
    let b = fasttext_topic_embedding(model, b);
    cosine_similarity(&a, &b)
}

fn print_embedding_check(model: &mut FastText, args: &Args) {
    let word_similarity = cosine_similarity(
        &model.get_word_vector("Asyl"),
        &model.get_word_vector("Asylrecht"),
    );
    let sentence_similarity = embedding_similarity(model, "Asyl", "Asylrecht");

    println!("FastText word-vector similarity Asyl <-> Asylrecht: {word_similarity:.4}");
    println!("FastText sentence-vector similarity Asyl <-> Asylrecht: {sentence_similarity:.4}");
    if args.use_fastembed {
        match fastembed_embedding_similarity("Asyl", "Asylrecht", &fastembed_cache_dir(args)) {
            Ok(similarity) => {
                println!(
                    "FastEmbed multilingual-e5-small similarity Asyl <-> Asylrecht: {similarity:.4}"
                )
            }
            Err(err) => println!("FastEmbed multilingual-e5-small similarity unavailable: {err}"),
        }
    }
    let layout = if args.use_fastembed {
        "local FastEmbed multilingual-e5-small embeddings"
    } else {
        "FastText sentence vectors"
    };
    println!("Graph layout uses {layout}.");
}

fn fastembed_embedding_similarity(
    lhs: &str,
    rhs: &str,
    cache_dir: &PathBuf,
) -> Result<f32, Box<dyn Error + Send + Sync>> {
    let mut model = load_fastembed_model(cache_dir)?;
    fastembed_embedding_similarity_with_model(&mut model, lhs, rhs)
}

fn fastembed_embedding_similarity_with_model(
    model: &mut TextEmbedding,
    lhs: &str,
    rhs: &str,
) -> Result<f32, Box<dyn Error + Send + Sync>> {
    let lhs = format!("query: {lhs}");
    let rhs = format!("query: {rhs}");
    let embeddings = model.embed(vec![lhs, rhs], None)?;

    Ok(cosine_similarity(&embeddings[0], &embeddings[1]))
}

fn fastembed_cache_dir(args: &Args) -> PathBuf {
    if args.fastembed_cache_dir.is_absolute() {
        args.fastembed_cache_dir.clone()
    } else {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(&args.fastembed_cache_dir)
    }
}

fn fasttext_topic_embedding(model: &mut FastText, topic: &str) -> Vec<f32> {
    model.get_sentence_vector(topic)
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a * norm_b)
    }
}

fn golden_angle(index: usize) -> f32 {
    index as f32 * 2.399_963_1
}
