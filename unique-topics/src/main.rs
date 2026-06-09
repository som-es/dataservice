use combx::connect_pg;
use fasttext::FastText;

use crate::unique_topics::{
    all_unique_eurovoc_topics, compare_ai_topics_with_unique_topics,
    hallucinated_ai_topics_by_model,
};

mod unique_topics;

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
    let mut model = FastText::load_model("models/cc.de.300.bin")?;

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

    for (query_topic, is_eurovoc) in &nodes {
        let query_vec = model.get_word_vector(query_topic);

        for (edge_topic, _) in &nodes {
            let edge_vec = model.get_word_vector(edge_topic);
            let weight = cosine_similarity(&query_vec, &edge_vec);
        }
    }

    Ok(())
}

use eframe::egui;
use egui_graphs::{
    DefaultEdgeShape, DefaultNodeShape, Graph, GraphView, LayoutRandom, LayoutStateRandom,
    SettingsInteraction, SettingsNavigation,
};
use petgraph::Undirected;
use petgraph::stable_graph::StableGraph;

type MyGraph = Graph<&'static str, f32, Undirected>;

type MyGraphView<'a> = GraphView<
    'a,
    &'static str,
    f32,
    Undirected,
    u32,
    DefaultNodeShape,
    DefaultEdgeShape,
    LayoutStateRandom,
    LayoutRandom,
>;

struct App {
    graph: MyGraph,
}

impl App {
    fn new(_: &eframe::CreationContext<'_>) -> Self {
        Self {
            graph: build_graph(),
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.heading("Simple undirected weighted graph");
            let navigation = SettingsNavigation::new()
                .with_fit_to_screen_enabled(true)
                .with_fit_to_screen_padding(0.15)
                .with_zoom_and_pan_enabled(true)
                .with_zoom_speed(0.1);
            let interactions = SettingsInteraction::new().with_dragging_enabled(false);
            let mut view: MyGraphView<'_> = GraphView::new(&mut self.graph)
                .with_interactions(&interactions)
                .with_navigations(&navigation);
            ui.add(&mut view);
        });
    }
}

fn build_graph() -> MyGraph {
    let base: StableGraph<&'static str, f32, Undirected> = StableGraph::default();
    let mut graph = Graph::from(&base);

    let democracy = graph.add_node_with_label("Demokratie", "Demokratie".to_owned());
    let parliament = graph.add_node_with_label("Parlament", "Parlament".to_owned());
    let government = graph.add_node_with_label("Regierung", "Regierung".to_owned());
    let football = graph.add_node_with_label("Fußball", "Fußball".to_owned());

    graph.add_edge_with_label(democracy, parliament, 0.91, "0.91".to_owned());
    graph.add_edge_with_label(parliament, government, 0.86, "0.86".to_owned());
    graph.add_edge_with_label(parliament, football, 0.21, "0.86".to_owned());
    graph.add_edge_with_label(democracy, government, 0.72, "0.72".to_owned());
    graph.add_edge_with_label(government, football, 0.18, "0.18".to_owned());

    graph
}

// fn main() -> eframe::Result {
//     eframe::run_native(
//         "weighted graph demo",
//         eframe::NativeOptions::default(),
//         Box::new(|cc| Ok(Box::new(App::new(cc)))),
//     )
// }
