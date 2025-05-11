use crate::domain::models::FileContext;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use log::{debug, info, warn};
use ratatui::{
    Frame, Terminal,
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};
use std::{
    io::{self},
    path::{Path, PathBuf},
    time::Duration,
};

// Tree node representation to store directory structure
enum TreeNode {
    Directory {
        name: String,
        children: Vec<TreeNode>,
        expanded: bool,
    },
    File {
        name: String,
        path: PathBuf,
        selected: bool,
    },
}

impl TreeNode {
    fn new_directory(name: String) -> Self {
        TreeNode::Directory {
            name,
            children: Vec::new(),
            expanded: true,
        }
    }

    fn new_file(name: String, path: PathBuf) -> Self {
        TreeNode::File {
            name,
            path,
            selected: false,
        }
    }

    fn is_file(&self) -> bool {
        matches!(self, TreeNode::File { .. })
    }

    fn get_display_name(&self) -> String {
        match self {
            TreeNode::Directory { name, .. } => name.clone(),
            TreeNode::File { name, .. } => name.clone(),
        }
    }

    fn toggle_selected(&mut self) {
        if let TreeNode::File { selected, .. } = self {
            *selected = !*selected;
        }
    }

    fn is_selected(&self) -> bool {
        match self {
            TreeNode::File { selected, .. } => *selected,
            _ => false,
        }
    }

    fn is_expanded(&self) -> bool {
        match self {
            TreeNode::Directory { expanded, .. } => *expanded,
            _ => false,
        }
    }
}


struct FlattenedTree {
    nodes: Vec<(TreeNode, usize)>,
    state: ListState,
}

impl FlattenedTree {
    fn new() -> Self {
        FlattenedTree {
            nodes: Vec::new(),
            state: ListState::default(),
        }
    }

    fn from_tree(root: &TreeNode) -> Self {
        let mut flattened = FlattenedTree::new();
        flattened.flatten_node(root, 0);

        if !flattened.nodes.is_empty() {
            flattened.state.select(Some(0));
        }

        flattened
    }

    fn flatten_node(&mut self, node: &TreeNode, depth: usize) {
        match node {
            TreeNode::Directory {
                name,
                children,
                expanded,
            } => {
                self.nodes.push((
                    TreeNode::Directory {
                        name: name.clone(),
                        children: Vec::new(),
                        expanded: *expanded,
                    },
                    depth,
                ));

                if *expanded {
                    for child in children {
                        self.flatten_node(child, depth + 1);
                    }
                }
            }
            TreeNode::File {
                name,
                path,
                selected,
            } => {
                self.nodes.push((
                    TreeNode::File {
                        name: name.clone(),
                        path: path.clone(),
                        selected: *selected,
                    },
                    depth,
                ));
            }
        }
    }

    fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.nodes.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.nodes.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn toggle_selected(&mut self) {
        if let Some(i) = self.state.selected() {
            let (node, _) = &mut self.nodes[i];
            if node.is_file() {
                node.toggle_selected();
            }
        }
    }

    fn selected_files_count(&self) -> usize {
        self.nodes
            .iter()
            .filter(|(node, _)| node.is_file() && node.is_selected())
            .count()
    }

    fn total_files_count(&self) -> usize {
        self.nodes.iter().filter(|(node, _)| node.is_file()).count()
    }

    fn get_selected_paths(&self) -> Vec<PathBuf> {
        self.nodes
            .iter()
            .filter_map(|(node, _)| {
                if let TreeNode::File { path, selected, .. } = node {
                    if *selected { Some(path.clone()) } else { None }
                } else {
                    None
                }
            })
            .collect()
    }

    fn select_all_files(&mut self) {
        for (node, _) in &mut self.nodes {
            if let TreeNode::File { selected, .. } = node {
                *selected = true;
            }
        }
    }

    fn deselect_all_files(&mut self) {
        for (node, _) in &mut self.nodes {
            if let TreeNode::File { selected, .. } = node {
                *selected = false;
            }
        }
    }
}

struct App {
    tree: TreeNode,
    flattened_tree: FlattenedTree,
    title: String,
    help_message: String,
}

impl App {
    fn new(files: Vec<PathBuf>, title: String) -> App {
        let mut root = TreeNode::new_directory("".to_string());

        for file_path in files {
            Self::add_path_to_tree(&mut root, &file_path);
        }

        let flattened_tree = FlattenedTree::from_tree(&root);

        App {
            tree: root,
            flattened_tree,
            title,
            help_message: String::from(
                "↑/↓: Navigate | Space: Toggle selection | Enter: Confirm | →/←: Expand/Collapse | q: Quit | a: Select all | n: Deselect all",
            ),
        }
    }

    fn add_path_to_tree(root: &mut TreeNode, path: &Path) {
        let components: Vec<_> = path
            .components()
            .map(|comp| comp.as_os_str().to_string_lossy().to_string())
            .collect();

        if components.is_empty() {
            return;
        }

        let mut current = root;
        let dirs_count = components.len() - 1;

        for i in 0..components.len() {
            let component_name = &components[i];

            if i == dirs_count {
                if let TreeNode::Directory { children, .. } = current {
                    let file_node = TreeNode::new_file(component_name.clone(), path.to_path_buf());
                    children.push(file_node);
                }
            } else {
                if let TreeNode::Directory { children, .. } = current {
                    let dir_pos = children.iter().position(|child| {
                        if let TreeNode::Directory { name, .. } = child {
                            name == component_name
                        } else {
                            false
                        }
                    });

                    if let Some(pos) = dir_pos {
                        current = &mut children[pos];
                    } else {
                        let new_dir = TreeNode::new_directory(component_name.clone());
                        children.push(new_dir);

                        let last_index = children.len() - 1;
                        current = &mut children[last_index];
                    }
                }
            }
        }
    }

    fn collapse_directory_by_name(&mut self, dir_name: &str) -> bool {
        fn find_and_collapse(node: &mut TreeNode, name: &str) -> bool {
            match node {
                TreeNode::Directory { name: node_name, expanded, children, .. } => {
                    if node_name == name {
                        *expanded = false;
                        return true;
                    }
                    
                    for child in children {
                        if find_and_collapse(child, name) {
                            return true;
                        }
                    }
                },
                _ => {}
            }
            false
        }
        
        let mut modified = false;
        if let TreeNode::Directory { children, .. } = &mut self.tree {
            for child in children {
                if find_and_collapse(child, dir_name) {
                    modified = true;
                    break;
                }
            }
        }
        
        modified
    }

    fn expand_directory_by_name(&mut self, dir_name: &str) -> bool {
        fn find_and_expand(node: &mut TreeNode, name: &str) -> bool {
            match node {
                TreeNode::Directory { name: node_name, expanded, children, .. } => {
                    if node_name == name {
                        *expanded = true;
                        return true;
                    }
                    
                    for child in children {
                        if find_and_expand(child, name) {
                            return true;
                        }
                    }
                },
                _ => {}
            }
            false
        }
        
        let mut modified = false;
        if let TreeNode::Directory { children, .. } = &mut self.tree {
            for child in children {
                if find_and_expand(child, dir_name) {
                    modified = true;
                    break;
                }
            }
        }
        
        modified
    }

    fn update_flattened_tree(&mut self) {
        let mut path_to_selected = Vec::new();
        if let Some(idx) = self.flattened_tree.state.selected() {
            if idx < self.flattened_tree.nodes.len() {
                let (node, _) = &self.flattened_tree.nodes[idx];
                if let TreeNode::Directory { name, .. } = node {
                    path_to_selected.push(name.clone());
                }
            }
        }

        self.flattened_tree = FlattenedTree::from_tree(&self.tree);

        if !path_to_selected.is_empty() && !self.flattened_tree.nodes.is_empty() {
            let dir_name = &path_to_selected[0];

            for (i, (node, _)) in self.flattened_tree.nodes.iter().enumerate() {
                if let TreeNode::Directory { name, .. } = node {
                    if name == dir_name {
                        self.flattened_tree.state.select(Some(i));
                        break;
                    }
                }
            }
        }
    }

    fn select_all(&mut self) {
        self.flattened_tree.select_all_files();
    }

    fn deselect_all(&mut self) {
        self.flattened_tree.deselect_all_files();
    }
}

fn ui<B: Backend>(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints(
            [
                Constraint::Length(1),
                Constraint::Min(1),
                Constraint::Length(1),
                Constraint::Length(1),
            ]
            .as_ref(),
        )
        .split(f.area());

    // Title
    let title = Paragraph::new(Span::styled(
        app.title.clone(),
        Style::default().add_modifier(Modifier::BOLD),
    ));
    f.render_widget(title, chunks[0]);

    // Files and directories tree
    let selected_style = Style::default()
        .bg(Color::Blue)
        .fg(Color::White)
        .add_modifier(Modifier::BOLD);

    let items: Vec<ListItem> = app
        .flattened_tree
        .nodes
        .iter()
        .enumerate()
        .map(|(i, (node, depth))| {
            let indent = "  ".repeat(*depth);
            let is_file = node.is_file();

            let prefix = if is_file {
                if node.is_selected() { "[✓] " } else { "[ ] " }
            } else {
                if node.is_expanded() { "▼ " } else { "► " }
            };

            let content = format!("{}{}{}", indent, prefix, node.get_display_name());
            let style = if app.flattened_tree.state.selected() == Some(i) {
                selected_style
            } else if is_file && node.is_selected() {
                Style::default().fg(Color::Green)
            } else if !is_file {
                Style::default().fg(Color::Blue)
            } else {
                Style::default()
            };

            ListItem::new(Span::styled(content, style))
        })
        .collect();

    let file_list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(format!(
            "Files ({} selected of {})",
            app.flattened_tree.selected_files_count(),
            app.flattened_tree.total_files_count()
        )))
        .highlight_style(selected_style);

    f.render_stateful_widget(file_list, chunks[1], &mut app.flattened_tree.state);

    // Controls help
    let controls = Paragraph::new(Span::styled(
        app.help_message.clone(),
        Style::default().fg(Color::DarkGray),
    ));
    f.render_widget(controls, chunks[3]);
}

pub fn select_files(
    files: Vec<PathBuf>,
    file_reader: impl Fn(&PathBuf) -> anyhow::Result<String>,
    auto: bool,
) -> anyhow::Result<Vec<FileContext>> {
    if files.is_empty() {
        info!("No files to select");
        return Ok(Vec::new());
    }

    debug!("Selecting from {} available files", files.len());

    if auto {
        info!("Auto-selecting all {} files", files.len());
        let mut selected_files = Vec::new();
        for path in files {
            debug!("Reading file: {}", path.display());
            match file_reader(&path) {
                Ok(content) => {
                    selected_files.push(FileContext { path, content });
                }
                Err(e) => {
                    warn!("Error reading file {}: {}", path.display(), e);
                }
            }
        }

        info!("Successfully loaded {} files", selected_files.len());
        return Ok(selected_files);
    }

    // Interactive TUI selection
    let selected_paths = run_tui(&files)?;

    let mut selected_files = Vec::new();
    for path in selected_paths {
        debug!("Reading file: {}", path.display());
        match file_reader(&path) {
            Ok(content) => {
                selected_files.push(FileContext {
                    path: path.clone(),
                    content,
                });
            }
            Err(e) => {
                warn!("Error reading file {}: {}", path.display(), e);
            }
        }
    }

    info!("Successfully loaded {} files", selected_files.len());
    Ok(selected_files)
}

fn run_tui(files: &[PathBuf]) -> anyhow::Result<Vec<PathBuf>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(
        files.to_vec(),
        "Select files to include in your LLM context".to_string(),
    );

    let result = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    match result {
        Ok(_) => {
            let selected = app.flattened_tree.get_selected_paths();
            info!("Selected {} files", selected.len());
            Ok(selected)
        }
        Err(err) => {
            warn!("Error during file selection: {}", err);
            Err(anyhow::anyhow!("Selection cancelled: {}", err))
        }
    }
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> anyhow::Result<()> {
    loop {
        terminal.draw(|f| ui::<B>(f, app))?;

        if crossterm::event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        if app.flattened_tree.selected_files_count() > 0 {
                            return Ok(());
                        } else {
                            return Err(anyhow::anyhow!("No files selected"));
                        }
                    }
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        return Err(anyhow::anyhow!("Selection cancelled"));
                    }
                    KeyCode::Char('a') => app.select_all(),
                    KeyCode::Char('n') => app.deselect_all(),
                    KeyCode::Char(' ') => {
                        app.flattened_tree.toggle_selected();
                    }
                    KeyCode::Right => {
                        let dir_name_to_expand = if let Some(i) = app.flattened_tree.state.selected() {
                            let (node, _) = &app.flattened_tree.nodes[i];
                            if !node.is_file() {
                                if let TreeNode::Directory { name, .. } = node {
                                    Some(name.clone())
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        };
                        
                        if let Some(name) = dir_name_to_expand {
                            if app.expand_directory_by_name(&name) {
                                let current_selection = app.flattened_tree.state.selected();
                                app.update_flattened_tree();
                                if let Some(idx) = current_selection {
                                    if idx < app.flattened_tree.nodes.len() {
                                        app.flattened_tree.state.select(Some(idx));
                                    } else if !app.flattened_tree.nodes.is_empty() {
                                        app.flattened_tree.state.select(Some(0));
                                    }
                                }
                            }
                        }
                    }
                    KeyCode::Left => {
                        let dir_name_to_collapse = if let Some(i) = app.flattened_tree.state.selected() {
                            let (node, _) = &app.flattened_tree.nodes[i];
                            if !node.is_file() {
                                if let TreeNode::Directory { name, .. } = node {
                                    Some(name.clone())
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        };
                        
                        if let Some(name) = dir_name_to_collapse {
                            if app.collapse_directory_by_name(&name) {
                                let current_selection = app.flattened_tree.state.selected();
                                app.update_flattened_tree();
                                if let Some(idx) = current_selection {
                                    if idx < app.flattened_tree.nodes.len() {
                                        app.flattened_tree.state.select(Some(idx));
                                    } else if !app.flattened_tree.nodes.is_empty() {
                                        app.flattened_tree.state.select(Some(0));
                                    }
                                }
                            }
                        }
                    }
                    KeyCode::Down => app.flattened_tree.next(),
                    KeyCode::Up => app.flattened_tree.previous(),
                    KeyCode::Enter => {
                        if app.flattened_tree.selected_files_count() > 0 {
                            return Ok(());
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    struct MockFileSystem {
        files: HashMap<PathBuf, String>,
    }

    impl MockFileSystem {
        fn new() -> Self {
            Self {
                files: HashMap::new(),
            }
        }

        fn add_file(&mut self, path: PathBuf, content: String) {
            self.files.insert(path, content);
        }

        fn read_file(&self, path: &PathBuf) -> anyhow::Result<String> {
            match self.files.get(path) {
                Some(content) => Ok(content.clone()),
                None => Err(anyhow::anyhow!("File not found")),
            }
        }
    }

    #[test]
    fn test_select_files_with_auto() {
        let mut mock_fs = MockFileSystem::new();
        mock_fs.add_file(PathBuf::from("file1.rs"), "content1".to_string());
        mock_fs.add_file(PathBuf::from("file2.rs"), "content2".to_string());

        let files = vec![PathBuf::from("file1.rs"), PathBuf::from("file2.rs")];

        let reader = |path: &PathBuf| mock_fs.read_file(path);

        let selected = select_files(files, reader, true).unwrap();

        assert_eq!(selected.len(), 2);
        assert_eq!(selected[0].content, "content1");
        assert_eq!(selected[1].content, "content2");
    }

    #[test]
    fn test_select_files_with_empty_input() {
        let files: Vec<PathBuf> = vec![];
        let reader = |_: &PathBuf| -> anyhow::Result<String> { Ok("".to_string()) };

        let selected = select_files(files, reader, true).unwrap();

        assert_eq!(selected.len(), 0);
    }

    #[test]
    fn test_select_files_with_read_error() {
        let files = vec![PathBuf::from("nonexistent.rs")];
        let reader =
            |_: &PathBuf| -> anyhow::Result<String> { Err(anyhow::anyhow!("File not found")) };

        let selected = select_files(files, reader, true).unwrap();

        assert_eq!(selected.len(), 0);
    }

    #[test]
    fn test_tree_structure() {
        let files = vec![
            PathBuf::from("src/main.rs"),
            PathBuf::from("src/lib.rs"),
            PathBuf::from("src/utils/helper.rs"),
        ];

        let app = App::new(files.clone(), "Test".to_string());

        if let TreeNode::Directory { children, .. } = &app.tree {
            assert_eq!(children.len(), 1);

            if let TreeNode::Directory { name, children, .. } = &children[0] {
                assert_eq!(name, "src");
                assert_eq!(children.len(), 3);

                let utils_dir = children.iter().find(|node| {
                    if let TreeNode::Directory { name, .. } = node {
                        name == "utils"
                    } else {
                        false
                    }
                });

                assert!(utils_dir.is_some());

                if let Some(TreeNode::Directory { children, .. }) = utils_dir {
                    assert_eq!(children.len(), 1);
                }
            }
        }
    }
}