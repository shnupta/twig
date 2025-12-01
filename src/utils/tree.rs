use crate::models::{Task, TaskStatus};
use crate::storage::Storage;

pub struct TreeNode {
    pub task: Task,
    pub children: Vec<TreeNode>,
}

impl TreeNode {
    pub fn build_forest(storage: &Storage) -> Vec<TreeNode> {
        let root_tasks = storage.get_root_tasks();
        root_tasks
            .into_iter()
            .map(|task| Self::build_tree(task, storage))
            .collect()
    }

    fn build_tree(task: &Task, storage: &Storage) -> TreeNode {
        let children_tasks = storage.get_children(task.id);
        let children = children_tasks
            .into_iter()
            .map(|child| Self::build_tree(child, storage))
            .collect();

        TreeNode {
            task: task.clone(),
            children,
        }
    }
}

pub fn format_tree(forest: &[TreeNode]) -> Vec<String> {
    let mut lines = Vec::new();
    for (i, node) in forest.iter().enumerate() {
        let is_last = i == forest.len() - 1;
        format_tree_node(node, "", is_last, &mut lines);
    }
    lines
}

fn format_tree_node(node: &TreeNode, prefix: &str, is_last: bool, lines: &mut Vec<String>) {
    let connector = if is_last { "└─" } else { "├─" };
    let status_icon = match node.task.status {
        TaskStatus::NotStarted => "○",
        TaskStatus::InProgress => "◐",
        TaskStatus::Completed => "●",
        TaskStatus::Cancelled => "✗",
    };

    let time_info = if node.task.total_time_seconds > 0 {
        format!(" [{}]", node.task.get_formatted_total_time())
    } else {
        String::new()
    };

    let estimate_info = if let Some(est) = node.task.get_formatted_estimate() {
        format!(" (~{})", est)
    } else {
        String::new()
    };

    let tags_info = if !node.task.tags.is_empty() {
        format!(
            " {}",
            node.task
                .tags
                .iter()
                .map(|t| format!("#{}", t))
                .collect::<Vec<_>>()
                .join(" ")
        )
    } else {
        String::new()
    };

    lines.push(format!(
        "{}{} {} {} [{}]{}{}{}",
        prefix,
        connector,
        status_icon,
        node.task.title,
        node.task.short_id(),
        time_info,
        estimate_info,
        tags_info
    ));

    let child_prefix = format!("{}{}", prefix, if is_last { "  " } else { "│ " });
    for (i, child) in node.children.iter().enumerate() {
        let child_is_last = i == node.children.len() - 1;
        format_tree_node(child, &child_prefix, child_is_last, lines);
    }
}
