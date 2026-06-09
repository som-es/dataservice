use clap::{Parser, ValueEnum};
use combx::connect_pg;
use eframe::egui;
use egui_graphs::{
    DefaultEdgeShape, DefaultNodeShape, DisplayEdge, DisplayNode, DrawContext, EdgeProps, Graph,
    GraphView, Layout, LayoutState, Node, SettingsInteraction, SettingsNavigation, SettingsStyle,
};
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use fasttext::FastText;
use petgraph::stable_graph::{IndexType, StableGraph};
use petgraph::{EdgeType, Undirected};
use std::collections::HashSet;

use crate::unique_topics::{all_unique_eurovoc_topics, hallucinated_ai_topics_by_model};

mod unique_topics;

const EDGE_THRESHOLD: f32 = 0.3;
const DISSIMILARITY_THRESHOLD: f32 = 0.08;
const TOP_RENDERED_EDGES_PER_NODE: usize = 16;
const TOP_REPULSION_PAIRS_PER_NODE: usize = 24;
const TOP_EUROVOC_GROUPS_PER_NODE: usize = 1;
const EUROVOC_GROUP_MIN_WEIGHT: f32 = 0.0;
const EUROVOC_GROUP_STRENGTH: f32 = 2.6;
const EUROVOC_ANCHOR_SPACING: f32 = 2.25;
const EUROVOC_GROUP_INNER_RADIUS: f32 = 220.0;
const EUROVOC_GROUP_RING_GAP: f32 = 115.0;
const EUROVOC_GROUP_SIMILARITY_SPREAD: f32 = 190.0;
const EUROVOC_GROUP_MAX_RADIUS: f32 = 760.0;
const MIN_EDGE_WIDTH: f32 = 0.8;
const MAX_EDGE_WIDTH: f32 = 4.5;
const LAYOUT_SCALE: f32 = 680.0;
const LAYOUT_EXPONENT: f32 = 2.0;
const RELAXATION_STEPS: usize = 140;
const RELAXATION_RATE: f32 = 0.09;
const EDGE_FORCE_EXPONENT: f32 = 2.0;
const SEMANTIC_REPULSION_RADIUS: f32 = 1_150.0;
const SEMANTIC_REPULSION_STRENGTH: f32 = 38.0;
const REPULSION_RADIUS: f32 = 520.0;
const REPULSION_STRENGTH: f32 = 18.0;
const MAX_RELAXATION_STEP: f32 = 85.0;
const MIN_EDGE_LENGTH: f32 = 150.0;
const MAX_EDGE_LENGTH: f32 = 940.0;
const GERMAN_STOP_WORDS: &[&str] = &[
    "aber", "als", "am", "an", "auch", "auf", "aus", "bei", "beim", "bis", "das", "dem", "den",
    "der", "des", "die", "durch", "ein", "eine", "einem", "einen", "einer", "eines", "für", "im",
    "in", "ist", "mit", "nach", "oder", "ohne", "sowie", "und", "vom", "von", "vor", "zu", "zum",
    "zur",
];

#[derive(Parser)]
struct Args {
    #[arg(long, value_enum, default_value_t = EmbeddingBackend::Fasttext)]
    embedding: EmbeddingBackend,
    #[arg(long, default_value = "models/cc.de.300.bin")]
    fasttext_model: String,
}

#[derive(Clone, Copy, ValueEnum)]
enum EmbeddingBackend {
    Fasttext,
    MultilingualE5Small,
}

type TopicGraph = Graph<String, f32, Undirected, u32, DefaultNodeShape, WeightedEdgeShape>;
type WeightedPair = (usize, usize, f32);

struct SimilarityEdges {
    attraction: Vec<WeightedPair>,
    eurovoc_groups: Vec<WeightedPair>,
    repulsion: Vec<WeightedPair>,
    rendered: Vec<WeightedPair>,
}

type TopicGraphView<'a> = GraphView<
    'a,
    String,
    f32,
    Undirected,
    u32,
    DefaultNodeShape,
    WeightedEdgeShape,
    NoopLayoutState,
    NoopLayout,
>;

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

#[tokio::main]
async fn main() -> combx::Result<()> {
    let args = Args::parse();
    let mut embedder = TopicEmbedder::new(args.embedding, &args.fasttext_model)?;

    let pg = connect_pg().await;

    let unique_topics = all_unique_eurovoc_topics(&pg).await?;
    let hallucinated_topics = hallucinated_ai_topics_by_model(&pg).await?;

    let nodes = collect_nodes(unique_topics, hallucinated_topics);

    let graph = build_graph(&nodes, &mut embedder)?;
    run_graph_app(graph).map_err(|err| std::io::Error::other(err.to_string()))?;

    Ok(())
}

fn collect_nodes(
    unique_topics: HashSet<String>,
    hallucinated_topics: Vec<(String, usize, HashSet<String>)>,
) -> Vec<(String, bool)> {
    let mut seen_topics = HashSet::new();
    let mut nodes = Vec::new();

    for topic in unique_topics {
        push_topic_node(&mut nodes, &mut seen_topics, topic, true);
    }

    for (_, _, topics) in hallucinated_topics {
        for topic in topics {
            push_topic_node(&mut nodes, &mut seen_topics, topic, false);
        }
    }

    nodes.sort_by(|(topic_a, is_eurovoc_a), (topic_b, is_eurovoc_b)| {
        is_eurovoc_b
            .cmp(is_eurovoc_a)
            .then_with(|| topic_a.cmp(topic_b))
    });

    nodes
}

fn push_topic_node(
    nodes: &mut Vec<(String, bool)>,
    seen_topics: &mut HashSet<String>,
    topic: String,
    is_eurovoc: bool,
) {
    let topic = topic.trim().to_owned();
    if topic.is_empty() {
        return;
    }

    if seen_topics.insert(topic_key(&topic)) {
        nodes.push((topic, is_eurovoc));
    }
}

fn topic_key(topic: &str) -> String {
    topic
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

enum TopicEmbedder {
    FastText(FastText),
    FastEmbed(TextEmbedding),
}

impl TopicEmbedder {
    fn new(backend: EmbeddingBackend, fasttext_model: &str) -> combx::Result<Self> {
        match backend {
            EmbeddingBackend::Fasttext => Ok(Self::FastText(FastText::load_model(fasttext_model)?)),
            EmbeddingBackend::MultilingualE5Small => Ok(Self::FastEmbed(TextEmbedding::try_new(
                InitOptions::new(EmbeddingModel::MultilingualE5Small)
                    .with_show_download_progress(true),
            )?)),
        }
    }

    fn embed_topics(&mut self, topics: &[String]) -> combx::Result<Vec<Vec<f32>>> {
        match self {
            Self::FastText(model) => Ok(topics
                .iter()
                .map(|topic| fasttext_topic_vector(model, topic))
                .collect()),
            Self::FastEmbed(model) => {
                let documents = topics
                    .iter()
                    .map(|topic| format!("passage: {topic}"))
                    .collect::<Vec<_>>();
                Ok(model.embed(documents, None)?)
            }
        }
    }
}

fn build_graph(
    nodes: &[(String, bool)],
    embedder: &mut TopicEmbedder,
) -> combx::Result<TopicGraph> {
    let base: StableGraph<String, f32, Undirected, u32> = StableGraph::default();
    let mut graph = Graph::from(&base);
    let topics = nodes
        .iter()
        .map(|(topic, _)| topic.clone())
        .collect::<Vec<_>>();
    let vectors = embedder.embed_topics(&topics)?;
    let is_eurovoc = nodes
        .iter()
        .map(|(_, is_eurovoc)| *is_eurovoc)
        .collect::<Vec<_>>();
    let similarity_edges = similarity_edges(&vectors, &is_eurovoc);
    let mut locations =
        eurovoc_guided_locations(&vectors, &is_eurovoc, &similarity_edges.eurovoc_groups);
    relax_locations(
        &mut locations,
        &is_eurovoc,
        &similarity_edges.attraction,
        &similarity_edges.eurovoc_groups,
        &similarity_edges.repulsion,
    );

    let graph_nodes = nodes
        .iter()
        .enumerate()
        .map(|(idx, (topic, is_eurovoc))| {
            graph.add_node_custom(topic.clone(), |node| {
                node.set_label(topic.clone());
                node.set_location(locations[idx]);
                if *is_eurovoc {
                    node.set_color(egui::Color32::from_rgb(76, 143, 255));
                } else {
                    node.set_color(egui::Color32::from_rgb(236, 159, 56));
                }
            })
        })
        .collect::<Vec<_>>();

    for (start_idx, end_idx, weight) in similarity_edges.rendered {
        graph.add_edge_with_label(
            graph_nodes[start_idx],
            graph_nodes[end_idx],
            weight,
            String::new(),
        );
    }

    Ok(graph)
}

fn content_tokens<'a>(tokens: &'a [&'a str]) -> Vec<&'a str> {
    let filtered_tokens = tokens
        .iter()
        .copied()
        .filter(|token| !is_german_stop_word(token))
        .collect::<Vec<_>>();

    if filtered_tokens.is_empty() {
        tokens.to_vec()
    } else {
        filtered_tokens
    }
}

fn fasttext_topic_vector(model: &FastText, topic: &str) -> Vec<f32> {
    let tokens = topic
        .split(|c: char| !c.is_alphanumeric())
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();

    if tokens.len() <= 1 {
        return model.get_word_vector(topic);
    }

    let averaged_tokens = content_tokens(&tokens);

    let mut vector = Vec::new();
    for token in &averaged_tokens {
        let token_vector = model.get_word_vector(token);
        if vector.is_empty() {
            vector.resize(token_vector.len(), 0.0);
        }

        for (idx, value) in token_vector.iter().enumerate() {
            vector[idx] += value;
        }
    }

    for value in &mut vector {
        *value /= averaged_tokens.len() as f32;
    }

    vector
}

fn is_german_stop_word(token: &str) -> bool {
    let lower = token.to_lowercase();
    GERMAN_STOP_WORDS.contains(&lower.as_str())
}

fn similarity_edges(vectors: &[Vec<f32>], is_eurovoc: &[bool]) -> SimilarityEdges {
    let mut attraction = Vec::new();
    let mut adjacency = vec![Vec::new(); vectors.len()];
    let mut repulsion_candidates = vec![Vec::new(); vectors.len()];
    let mut eurovoc_candidates = vec![Vec::new(); vectors.len()];

    for (query_idx, query_vec) in vectors.iter().enumerate() {
        for (edge_idx, edge_vec) in vectors.iter().enumerate().skip(query_idx + 1) {
            let weight = cosine_similarity(&query_vec, &edge_vec);
            collect_eurovoc_candidate(
                &mut eurovoc_candidates,
                is_eurovoc,
                query_idx,
                edge_idx,
                weight,
            );

            if weight >= EDGE_THRESHOLD {
                attraction.push((query_idx, edge_idx, weight));
                adjacency[query_idx].push((edge_idx, weight));
                adjacency[edge_idx].push((query_idx, weight));
            } else if weight <= DISSIMILARITY_THRESHOLD {
                repulsion_candidates[query_idx].push((edge_idx, weight));
                repulsion_candidates[edge_idx].push((query_idx, weight));
            }
        }
    }

    let eurovoc_groups = eurovoc_group_edges(&mut eurovoc_candidates, is_eurovoc);

    let mut rendered_edge_pairs = HashSet::new();
    let mut rendered_edges = Vec::new();
    for (query_idx, neighbors) in adjacency.iter_mut().enumerate() {
        neighbors.sort_by(|(_, a), (_, b)| b.total_cmp(a));
        for &(edge_idx, weight) in neighbors.iter().take(TOP_RENDERED_EDGES_PER_NODE) {
            let edge_pair = if query_idx < edge_idx {
                (query_idx, edge_idx)
            } else {
                (edge_idx, query_idx)
            };
            if rendered_edge_pairs.insert(edge_pair) {
                rendered_edges.push((edge_pair.0, edge_pair.1, weight));
            }
        }
    }

    let mut repulsion_edge_pairs = HashSet::new();
    let mut repulsion_edges = Vec::new();
    for (query_idx, neighbors) in repulsion_candidates.iter_mut().enumerate() {
        neighbors.sort_by(|(_, a), (_, b)| a.total_cmp(b));
        for &(edge_idx, weight) in neighbors.iter().take(TOP_REPULSION_PAIRS_PER_NODE) {
            let edge_pair = if query_idx < edge_idx {
                (query_idx, edge_idx)
            } else {
                (edge_idx, query_idx)
            };
            if repulsion_edge_pairs.insert(edge_pair) {
                repulsion_edges.push((edge_pair.0, edge_pair.1, weight));
            }
        }
    }

    SimilarityEdges {
        attraction,
        eurovoc_groups,
        repulsion: repulsion_edges,
        rendered: rendered_edges,
    }
}

fn collect_eurovoc_candidate(
    eurovoc_candidates: &mut [Vec<(usize, f32)>],
    is_eurovoc: &[bool],
    query_idx: usize,
    edge_idx: usize,
    weight: f32,
) {
    match (is_eurovoc[query_idx], is_eurovoc[edge_idx]) {
        (true, false) => eurovoc_candidates[edge_idx].push((query_idx, weight)),
        (false, true) => eurovoc_candidates[query_idx].push((edge_idx, weight)),
        _ => {}
    }
}

fn eurovoc_group_edges(
    eurovoc_candidates: &mut [Vec<(usize, f32)>],
    is_eurovoc: &[bool],
) -> Vec<WeightedPair> {
    let mut group_edges = Vec::new();

    for (topic_idx, candidates) in eurovoc_candidates.iter_mut().enumerate() {
        if is_eurovoc[topic_idx] {
            continue;
        }

        candidates.sort_by(|(_, a), (_, b)| b.total_cmp(a));
        for &(eurovoc_idx, weight) in candidates.iter().take(TOP_EUROVOC_GROUPS_PER_NODE) {
            group_edges.push((eurovoc_idx, topic_idx, weight));
        }
    }

    group_edges
}

struct App {
    graph: TopicGraph,
}

impl App {
    fn new(graph: TopicGraph) -> Self {
        Self { graph }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        egui::CentralPanel::default().show_inside(ui, |ui| {
            let navigation = SettingsNavigation::new()
                .with_fit_to_screen_enabled(false)
                .with_zoom_and_pan_enabled(true)
                .with_zoom_speed(0.08);
            let interactions = SettingsInteraction::new().with_dragging_enabled(false);
            let style = SettingsStyle::new().with_labels_always(false);
            let mut view: TopicGraphView<'_> = GraphView::new(&mut self.graph)
                .with_interactions(&interactions)
                .with_navigations(&navigation)
                .with_styles(&style);
            ui.add(&mut view);
        });
    }
}

fn run_graph_app(graph: TopicGraph) -> eframe::Result {
    eframe::run_native(
        "topic similarity graph",
        eframe::NativeOptions::default(),
        Box::new(|_| Ok(Box::new(App::new(graph)))),
    )
}

fn eurovoc_guided_locations(
    vectors: &[Vec<f32>],
    is_eurovoc: &[bool],
    eurovoc_groups: &[WeightedPair],
) -> Vec<egui::Pos2> {
    if vectors.is_empty() {
        return Vec::new();
    }

    let fallback_locations = semantic_locations(vectors);
    let eurovoc_indices = is_eurovoc
        .iter()
        .enumerate()
        .filter_map(|(idx, is_eurovoc)| is_eurovoc.then_some(idx))
        .collect::<Vec<_>>();

    if eurovoc_indices.is_empty() {
        return fallback_locations;
    }

    let eurovoc_vectors = eurovoc_indices
        .iter()
        .map(|&idx| vectors[idx].clone())
        .collect::<Vec<_>>();
    let eurovoc_locations = semantic_locations(&eurovoc_vectors);
    let mut locations = fallback_locations;

    for (&node_idx, location) in eurovoc_indices.iter().zip(eurovoc_locations) {
        locations[node_idx] = (location.to_vec2() * EUROVOC_ANCHOR_SPACING).to_pos2();
    }

    let mut group_by_topic = vec![None; vectors.len()];
    for &(eurovoc_idx, topic_idx, weight) in eurovoc_groups {
        group_by_topic[topic_idx] = Some((eurovoc_idx, weight));
    }

    let mut group_member_counts = vec![0usize; vectors.len()];
    for (topic_idx, group) in group_by_topic.iter().enumerate() {
        if is_eurovoc[topic_idx] {
            continue;
        }

        let Some((eurovoc_idx, weight)) = group else {
            continue;
        };
        let slot = group_member_counts[*eurovoc_idx];
        group_member_counts[*eurovoc_idx] += 1;
        let angle = group_angle(*eurovoc_idx, slot);
        let radius = eurovoc_group_initial_radius(slot, *weight);
        let offset = egui::vec2(angle.cos(), angle.sin()) * radius;
        locations[topic_idx] = (locations[*eurovoc_idx].to_vec2() + offset).to_pos2();
    }

    locations
}

fn group_angle(eurovoc_idx: usize, slot: usize) -> f32 {
    const GOLDEN_ANGLE: f32 = 2.3999631;
    slot as f32 * GOLDEN_ANGLE + eurovoc_idx as f32 * 0.47
}

fn eurovoc_group_initial_radius(slot: usize, weight: f32) -> f32 {
    let slot_radius = (slot as f32).sqrt() * EUROVOC_GROUP_RING_GAP;
    let similarity_radius =
        (1.0 - normalized_eurovoc_group_similarity(weight)) * EUROVOC_GROUP_SIMILARITY_SPREAD;
    (EUROVOC_GROUP_INNER_RADIUS + slot_radius + similarity_radius).min(EUROVOC_GROUP_MAX_RADIUS)
}

fn semantic_locations(vectors: &[Vec<f32>]) -> Vec<egui::Pos2> {
    if vectors.is_empty() {
        return Vec::new();
    }

    let normalized = vectors
        .iter()
        .map(|v| normalized_vector(v))
        .collect::<Vec<_>>();
    let mean = vector_mean(&normalized);
    let centered = normalized
        .iter()
        .map(|v| {
            v.iter()
                .zip(&mean)
                .map(|(x, mean)| x - mean)
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    let pc1 = principal_component(&centered, None);
    let pc2 = principal_component(&centered, Some(&pc1));
    let raw_locations = centered
        .iter()
        .map(|v| egui::vec2(dot_product(v, &pc1), dot_product(v, &pc2)))
        .collect::<Vec<_>>();

    scale_locations(&raw_locations)
}

fn normalized_vector(vector: &[f32]) -> Vec<f32> {
    let norm = vector.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm == 0.0 {
        return vec![0.0; vector.len()];
    }

    vector.iter().map(|x| x / norm).collect()
}

fn vector_mean(vectors: &[Vec<f32>]) -> Vec<f32> {
    let mut mean = vec![0.0; vectors[0].len()];
    for vector in vectors {
        for (idx, value) in vector.iter().enumerate() {
            mean[idx] += value;
        }
    }

    for value in &mut mean {
        *value /= vectors.len() as f32;
    }

    mean
}

fn principal_component(vectors: &[Vec<f32>], previous_component: Option<&[f32]>) -> Vec<f32> {
    let dims = vectors[0].len();
    let mut component = (0..dims)
        .map(|idx| ((idx * 37 + 17) % 101) as f32 / 100.0 - 0.5)
        .collect::<Vec<_>>();
    normalize_in_place(&mut component);

    for _ in 0..40 {
        let mut next = vec![0.0; dims];
        for vector in vectors {
            let projection = dot_product(vector, &component);
            for (idx, value) in vector.iter().enumerate() {
                next[idx] += projection * value;
            }
        }

        if let Some(previous) = previous_component {
            let previous_projection = dot_product(&next, previous);
            for (idx, value) in previous.iter().enumerate() {
                next[idx] -= previous_projection * value;
            }
        }

        normalize_in_place(&mut next);
        component = next;
    }

    component
}

fn normalize_in_place(vector: &mut [f32]) {
    let norm = vector.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm == 0.0 {
        return;
    }

    for value in vector {
        *value /= norm;
    }
}

fn dot_product(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

fn scale_locations(locations: &[egui::Vec2]) -> Vec<egui::Pos2> {
    let center = locations
        .iter()
        .copied()
        .fold(egui::Vec2::ZERO, |sum, value| sum + value)
        / locations.len() as f32;
    let variance = locations
        .iter()
        .map(|location| (*location - center).length_sq())
        .sum::<f32>()
        / locations.len() as f32;
    let std_dev = variance.sqrt().max(0.001);

    locations
        .iter()
        .map(|location| {
            let normalized = (*location - center) / std_dev;
            let radius = normalized.length();
            if radius == 0.0 {
                return egui::Pos2::ZERO;
            }

            let direction = normalized / radius;
            (direction * radius.powf(LAYOUT_EXPONENT) * LAYOUT_SCALE).to_pos2()
        })
        .collect()
}

fn relax_locations(
    locations: &mut [egui::Pos2],
    is_eurovoc: &[bool],
    attraction_edges: &[WeightedPair],
    eurovoc_groups: &[WeightedPair],
    repulsion_edges: &[WeightedPair],
) {
    for _ in 0..RELAXATION_STEPS {
        let mut displacements = vec![egui::Vec2::ZERO; locations.len()];

        apply_semantic_attraction(locations, attraction_edges, &mut displacements);
        apply_eurovoc_group_attraction(locations, eurovoc_groups, &mut displacements);
        apply_semantic_repulsion(locations, repulsion_edges, &mut displacements);
        apply_local_repulsion(locations, &mut displacements);

        for ((location, displacement), is_eurovoc) in locations
            .iter_mut()
            .zip(displacements)
            .zip(is_eurovoc.iter().copied())
        {
            if is_eurovoc {
                continue;
            }

            *location += capped_displacement(displacement);
        }
    }
}

fn apply_eurovoc_group_attraction(
    locations: &[egui::Pos2],
    eurovoc_groups: &[WeightedPair],
    displacements: &mut [egui::Vec2],
) {
    for &(eurovoc_idx, topic_idx, weight) in eurovoc_groups {
        let eurovoc = locations[eurovoc_idx];
        let topic = locations[topic_idx];
        let delta = topic - eurovoc;
        let distance = delta.length().max(0.001);
        let target_distance = eurovoc_group_target_distance(weight);
        let force = normalized_eurovoc_group_similarity(weight).powf(1.5)
            * RELAXATION_RATE
            * EUROVOC_GROUP_STRENGTH;
        let adjustment = delta / distance * ((distance - target_distance) * force);

        displacements[topic_idx] -= adjustment;
    }
}

fn apply_semantic_attraction(
    locations: &[egui::Pos2],
    attraction_edges: &[WeightedPair],
    displacements: &mut [egui::Vec2],
) {
    for &(start_idx, end_idx, weight) in attraction_edges {
        let start = locations[start_idx];
        let end = locations[end_idx];
        let delta = end - start;
        let distance = delta.length().max(0.001);
        let target_distance = edge_target_distance(weight);
        let force = normalized_similarity(weight).powf(EDGE_FORCE_EXPONENT);
        let adjustment =
            delta / distance * ((distance - target_distance) * RELAXATION_RATE * force);

        displacements[start_idx] += adjustment;
        displacements[end_idx] -= adjustment;
    }
}

fn apply_semantic_repulsion(
    locations: &[egui::Pos2],
    repulsion_edges: &[WeightedPair],
    displacements: &mut [egui::Vec2],
) {
    for &(start_idx, end_idx, weight) in repulsion_edges {
        let start = locations[start_idx];
        let end = locations[end_idx];
        let delta = end - start;
        let distance = delta.length();
        if distance >= SEMANTIC_REPULSION_RADIUS {
            continue;
        }

        let direction = if distance == 0.0 {
            deterministic_direction(start_idx, end_idx)
        } else {
            delta / distance
        };
        let proximity = (SEMANTIC_REPULSION_RADIUS - distance) / SEMANTIC_REPULSION_RADIUS;
        let dissimilarity = normalized_dissimilarity(weight);
        let force =
            proximity * proximity * dissimilarity * dissimilarity * SEMANTIC_REPULSION_STRENGTH;

        displacements[start_idx] -= direction * force;
        displacements[end_idx] += direction * force;
    }
}

fn apply_local_repulsion(locations: &[egui::Pos2], displacements: &mut [egui::Vec2]) {
    for start_idx in 0..locations.len() {
        for end_idx in (start_idx + 1)..locations.len() {
            let delta = locations[end_idx] - locations[start_idx];
            let distance = delta.length();
            if distance >= REPULSION_RADIUS {
                continue;
            }

            let direction = if distance == 0.0 {
                deterministic_direction(start_idx, end_idx)
            } else {
                delta / distance
            };
            let proximity = (REPULSION_RADIUS - distance) / REPULSION_RADIUS;
            let force = proximity * proximity * REPULSION_STRENGTH;

            displacements[start_idx] -= direction * force;
            displacements[end_idx] += direction * force;
        }
    }
}

fn deterministic_direction(start_idx: usize, end_idx: usize) -> egui::Vec2 {
    let angle = ((start_idx * 31 + end_idx * 17) % 360) as f32 * std::f32::consts::TAU / 360.0;
    egui::vec2(angle.cos(), angle.sin())
}

fn capped_displacement(displacement: egui::Vec2) -> egui::Vec2 {
    let length = displacement.length();
    if length <= MAX_RELAXATION_STEP {
        displacement
    } else {
        displacement / length * MAX_RELAXATION_STEP
    }
}

fn edge_target_distance(weight: f32) -> f32 {
    let normalized = normalized_similarity(weight);
    MAX_EDGE_LENGTH - normalized * (MAX_EDGE_LENGTH - MIN_EDGE_LENGTH)
}

fn eurovoc_group_target_distance(weight: f32) -> f32 {
    let normalized = normalized_eurovoc_group_similarity(weight);
    MAX_EDGE_LENGTH * 0.65 - normalized * (MAX_EDGE_LENGTH * 0.65 - MIN_EDGE_LENGTH * 0.85)
}

fn normalized_similarity(weight: f32) -> f32 {
    ((weight - EDGE_THRESHOLD) / (1.0 - EDGE_THRESHOLD)).clamp(0.0, 1.0)
}

fn normalized_eurovoc_group_similarity(weight: f32) -> f32 {
    ((weight - EUROVOC_GROUP_MIN_WEIGHT) / (1.0 - EUROVOC_GROUP_MIN_WEIGHT)).clamp(0.0, 1.0)
}

fn normalized_dissimilarity(weight: f32) -> f32 {
    ((DISSIMILARITY_THRESHOLD - weight) / (DISSIMILARITY_THRESHOLD + 1.0)).clamp(0.0, 1.0)
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
struct NoopLayoutState;

#[derive(Debug, Default)]
struct NoopLayout {
    state: NoopLayoutState,
}

impl LayoutState for NoopLayoutState {}

impl Layout<NoopLayoutState> for NoopLayout {
    fn from_state(state: NoopLayoutState) -> impl Layout<NoopLayoutState> {
        Self { state }
    }

    fn next<N, E, Ty, Ix, Dn, De>(
        &mut self,
        _graph: &mut Graph<N, E, Ty, Ix, Dn, De>,
        _ui: &egui::Ui,
    ) where
        N: Clone,
        E: Clone,
        Ty: EdgeType,
        Ix: IndexType,
        Dn: DisplayNode<N, E, Ty, Ix>,
        De: DisplayEdge<N, E, Ty, Ix, Dn>,
    {
    }

    fn state(&self) -> NoopLayoutState {
        self.state.clone()
    }
}

#[derive(Clone)]
struct WeightedEdgeShape {
    default_impl: DefaultEdgeShape,
}

impl From<EdgeProps<f32>> for WeightedEdgeShape {
    fn from(props: EdgeProps<f32>) -> Self {
        let mut default_impl = DefaultEdgeShape::from(props.clone());
        default_impl.width = edge_width(props.payload);

        Self { default_impl }
    }
}

impl<N, Ty, Ix, D> DisplayEdge<N, f32, Ty, Ix, D> for WeightedEdgeShape
where
    N: Clone,
    Ty: EdgeType,
    Ix: IndexType,
    D: DisplayNode<N, f32, Ty, Ix>,
{
    fn shapes(
        &mut self,
        start: &Node<N, f32, Ty, Ix, D>,
        end: &Node<N, f32, Ty, Ix, D>,
        ctx: &DrawContext,
    ) -> Vec<egui::Shape> {
        self.default_impl.shapes(start, end, ctx)
    }

    fn update(&mut self, state: &EdgeProps<f32>) {
        <DefaultEdgeShape as DisplayEdge<N, f32, Ty, Ix, D>>::update(&mut self.default_impl, state);
        self.default_impl.width = edge_width(state.payload);
    }

    fn is_inside(
        &self,
        start: &Node<N, f32, Ty, Ix, D>,
        end: &Node<N, f32, Ty, Ix, D>,
        pos: egui::Pos2,
    ) -> bool {
        self.default_impl.is_inside(start, end, pos)
    }

    fn extra_bounds(
        &self,
        start: &Node<N, f32, Ty, Ix, D>,
        end: &Node<N, f32, Ty, Ix, D>,
    ) -> Option<(egui::Pos2, egui::Pos2)> {
        self.default_impl.extra_bounds(start, end)
    }
}

fn edge_width(weight: f32) -> f32 {
    let normalized = ((weight - EDGE_THRESHOLD) / (1.0 - EDGE_THRESHOLD)).clamp(0.0, 1.0);
    MIN_EDGE_WIDTH + normalized * (MAX_EDGE_WIDTH - MIN_EDGE_WIDTH)
}
