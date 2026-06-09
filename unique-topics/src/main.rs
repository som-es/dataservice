use combx::connect_pg;
use eframe::egui;
use egui_graphs::{
    DefaultEdgeShape, DefaultNodeShape, DisplayEdge, DisplayNode, DrawContext, EdgeProps, Graph,
    GraphView, Layout, LayoutState, Node, SettingsInteraction, SettingsNavigation, SettingsStyle,
};
use fasttext::FastText;
use petgraph::stable_graph::{IndexType, StableGraph};
use petgraph::{EdgeType, Undirected};
use std::collections::HashSet;

use crate::unique_topics::{all_unique_eurovoc_topics, hallucinated_ai_topics_by_model};

mod unique_topics;

const EDGE_THRESHOLD: f32 = 0.3;
const TOP_RENDERED_EDGES_PER_NODE: usize = 16;
const MIN_EDGE_WIDTH: f32 = 0.8;
const MAX_EDGE_WIDTH: f32 = 4.5;
const LAYOUT_SCALE: f32 = 680.0;
const LAYOUT_EXPONENT: f32 = 2.0;
const RELAXATION_STEPS: usize = 140;
const RELAXATION_RATE: f32 = 0.09;
const EDGE_FORCE_EXPONENT: f32 = 2.0;
const MIN_EDGE_LENGTH: f32 = 150.0;
const MAX_EDGE_LENGTH: f32 = 940.0;
const GERMAN_STOP_WORDS: &[&str] = &[
    "aber", "als", "am", "an", "auch", "auf", "aus", "bei", "beim", "bis", "das", "dem", "den",
    "der", "des", "die", "durch", "ein", "eine", "einem", "einen", "einer", "eines", "für", "im",
    "in", "ist", "mit", "nach", "oder", "ohne", "sowie", "und", "vom", "von", "vor", "zu", "zum",
    "zur",
];

type TopicGraph = Graph<String, f32, Undirected, u32, DefaultNodeShape, WeightedEdgeShape>;

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
    let model = FastText::load_model("models/cc.de.300.bin")?;

    let pg = connect_pg().await;

    let unique_topics = all_unique_eurovoc_topics(&pg).await?;
    let hallucinated_topics = hallucinated_ai_topics_by_model(&pg).await?;

    let nodes = unique_topics
        .into_iter()
        .map(|topic| (topic, true))
        .chain(
            hallucinated_topics
                .into_iter()
                .map(|(_, _, topics)| topics.into_iter().map(|topic| (topic, false)))
                .flatten(),
        )
        .collect::<Vec<_>>();

    let graph = build_graph(&nodes, &model);
    run_graph_app(graph).map_err(|err| std::io::Error::other(err.to_string()))?;

    Ok(())
}

fn build_graph(nodes: &[(String, bool)], model: &FastText) -> TopicGraph {
    let base: StableGraph<String, f32, Undirected, u32> = StableGraph::default();
    let mut graph = Graph::from(&base);
    let vectors = nodes
        .iter()
        .map(|(topic, _)| topic_vector(model, topic))
        .collect::<Vec<_>>();
    let mut locations = semantic_locations(&vectors);
    let (layout_edges, rendered_edges) = similarity_edges(&vectors);
    relax_locations(&mut locations, &layout_edges);

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

    for (start_idx, end_idx, weight) in rendered_edges {
        graph.add_edge_with_label(
            graph_nodes[start_idx],
            graph_nodes[end_idx],
            weight,
            String::new(),
        );
    }

    graph
}

fn topic_vector(model: &FastText, topic: &str) -> Vec<f32> {
    let tokens = topic
        .split(|c: char| !c.is_alphanumeric())
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();

    if tokens.len() <= 1 {
        return model.get_word_vector(topic);
    }

    let filtered_tokens = tokens
        .iter()
        .copied()
        .filter(|token| !is_german_stop_word(token))
        .collect::<Vec<_>>();
    let averaged_tokens = if filtered_tokens.is_empty() {
        &tokens
    } else {
        &filtered_tokens
    };

    let mut vector = Vec::new();
    for token in averaged_tokens {
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

fn similarity_edges(vectors: &[Vec<f32>]) -> (Vec<(usize, usize, f32)>, Vec<(usize, usize, f32)>) {
    let mut layout_edges = Vec::new();
    let mut adjacency = vec![Vec::new(); vectors.len()];

    for (query_idx, query_vec) in vectors.iter().enumerate() {
        for (edge_idx, edge_vec) in vectors.iter().enumerate().skip(query_idx + 1) {
            let weight = cosine_similarity(&query_vec, &edge_vec);
            if weight >= EDGE_THRESHOLD {
                layout_edges.push((query_idx, edge_idx, weight));
                adjacency[query_idx].push((edge_idx, weight));
                adjacency[edge_idx].push((query_idx, weight));
            }
        }
    }

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

    (layout_edges, rendered_edges)
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

fn relax_locations(locations: &mut [egui::Pos2], edges: &[(usize, usize, f32)]) {
    for _ in 0..RELAXATION_STEPS {
        for &(start_idx, end_idx, weight) in edges {
            let start = locations[start_idx];
            let end = locations[end_idx];
            let delta = end - start;
            let distance = delta.length().max(0.001);
            let target_distance = edge_target_distance(weight);
            let force = normalized_similarity(weight).powf(EDGE_FORCE_EXPONENT);
            let adjustment =
                delta / distance * ((distance - target_distance) * RELAXATION_RATE * force);

            locations[start_idx] += adjustment;
            locations[end_idx] -= adjustment;
        }
    }
}

fn edge_target_distance(weight: f32) -> f32 {
    let normalized = normalized_similarity(weight);
    MAX_EDGE_LENGTH - normalized * (MAX_EDGE_LENGTH - MIN_EDGE_LENGTH)
}

fn normalized_similarity(weight: f32) -> f32 {
    ((weight - EDGE_THRESHOLD) / (1.0 - EDGE_THRESHOLD)).clamp(0.0, 1.0)
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
