use combx::connect_pg;
use eframe::egui;
use egui_graphs::{
    DefaultEdgeShape, DefaultNodeShape, DisplayEdge, DisplayNode, DrawContext, EdgeProps, Graph,
    GraphView, Layout, LayoutRandom, LayoutState, LayoutStateRandom, Node, SettingsInteraction,
    SettingsNavigation, SettingsStyle,
};
use fasttext::FastText;
use petgraph::stable_graph::{IndexType, StableGraph};
use petgraph::{EdgeType, Undirected};
use std::collections::HashSet;

use crate::unique_topics::{all_unique_eurovoc_topics, hallucinated_ai_topics_by_model};

mod unique_topics;

const GERMAN_STOP_WORDS: &[&str] = &[
    "aber", "als", "am", "an", "auch", "auf", "aus", "bei", "beim", "bis", "das", "dem", "den",
    "der", "des", "die", "durch", "ein", "eine", "einem", "einen", "einer", "eines", "für", "im",
    "in", "ist", "mit", "nach", "oder", "ohne", "sowie", "und", "vom", "von", "vor", "zu", "zum",
    "zur",
];

// type TopicGraph = Graph<String, f32, Undirected, u32, DefaultNodeShape, WeightedEdgeShape>;

// type TopicGraphView<'a> = GraphView<
//     'a,
//     String,
//     f32,
//     Undirected,
//     u32,
//     DefaultNodeShape,
//     WeightedEdgeShape,
//     NoopLayoutState,
//     NoopLayout,
// >;

type TopicGraph = Graph<String, f32, Undirected>;

type TopicGraphView<'a> = GraphView<
    'a,
    String,
    f32,
    Undirected,
    u32,
    DefaultNodeShape,
    DefaultEdgeShape,
    LayoutStateRandom,
    LayoutRandom,
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
    // let base: StableGraph<String, f32, Undirected, u32> = StableGraph::default();
    let base: StableGraph<String, f32, Undirected> = StableGraph::default();
    let mut graph = Graph::from(&base);

    let graph_nodes = nodes
        .iter()
        .map(|(node_topic, is_eurovoc)| {
            let node_idx = graph.add_node_custom(node_topic.clone(), |node| {
                node.set_label(node_topic.clone());
                if *is_eurovoc {
                    node.set_color(egui::Color32::from_rgb(76, 143, 255));
                } else {
                    node.set_color(egui::Color32::from_rgb(236, 159, 56));
                }
            });
            (node_idx, *is_eurovoc, node_topic)
        })
        .collect::<Vec<_>>();

    for (node, is_eurovoc, node_topic) in &graph_nodes {
        if !is_eurovoc {
            continue;
        }

        let node_vector = topic_vector(model, node_topic);
        for (other_node, is_eurovoc, other_node_topic) in &graph_nodes {
            if *is_eurovoc || node == other_node {
                continue;
            }
            let other_vector = topic_vector(model, other_node_topic);
            let weight = cosine_similarity(&node_vector, &other_vector);
            graph.add_edge_with_label(*node, *other_node, weight, String::new());
        }
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
