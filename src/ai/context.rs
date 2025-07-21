use crate::core::constants::CLEANABLE_PATTERNS;

/// Context-aware AI prompts for DevTidy
pub struct DevTidyContext {
  conversation_history: Vec<(String, String)>, // (user_input, ai_response)
}

impl DevTidyContext {
  pub fn new() -> Self {
    Self {
      conversation_history: Vec::new(),
    }
  }

  /// Add a conversation exchange to history
  pub fn add_exchange(&mut self, user_input: String, ai_response: String) {
    self.conversation_history.push((user_input, ai_response));

    // Keep only last 5 exchanges to avoid token limits
    if self.conversation_history.len() > 5 {
      self.conversation_history.remove(0);
    }
  }

  /// Create a context-aware prompt for explaining files/folders
  pub fn create_explain_prompt(&self, folder_name: &str) -> String {
    let app_context = self.get_app_context();
    let pattern_info = self.check_cleanable_pattern(folder_name);
    let conversation_context = self.get_conversation_context();

    format!(
            "{app_context}\n\n{pattern_info}\n\nFolder/file to explain: '{folder_name}'\n\n{conversation_context}\n\nExplain what this folder/file is used for and whether DevTidy can safely delete it. Be specific and reference the pattern information above if applicable. Keep response concise (1-2 sentences)."
        )
  }

  /// Create a context-aware prompt for cleanup suggestions  
  pub fn create_suggest_prompt(&self, folder_name: &str, size: &str) -> String {
    let app_context = self.get_app_context();
    let pattern_info = self.check_cleanable_pattern(folder_name);

    format!(
            "{app_context}\n\n{pattern_info}\n\nFolder: '{folder_name}' (size: {size})\n\nBased on DevTidy's patterns above, can this folder be safely deleted? Give a clear yes/no answer with brief reasoning."
        )
  }

  /// Create a context-aware prompt for chat interactions
  pub fn create_chat_prompt(&self, user_input: &str) -> String {
    let app_context = self.get_app_context();
    let conversation_context = self.get_conversation_context();

    format!(
            "{app_context}\n\n{conversation_context}\n\nUser question: {user_input}\n\nAnswer as DevTidy's AI assistant. If the question is about files/folders, check if they match any cleanable patterns and advise accordingly. Be helpful and specific."
        )
  }

  /// Get DevTidy application context
  fn get_app_context(&self) -> String {
    let patterns_list = CLEANABLE_PATTERNS
      .iter()
      .map(|(pattern, desc)| format!("  - {}: {}", pattern, desc))
      .collect::<Vec<_>>()
      .join("\n");

    format!(
            "You are DevTidy's AI assistant. DevTidy is a development artifact cleaner that helps developers free up disk space by removing build artifacts, caches, and temporary files.\n\nDevTidy recognizes these cleanable patterns:\n{patterns_list}\n\nWhen users ask about files/folders, check if they match these patterns to give informed deletion advice."
        )
  }

  /// Check if a folder/file matches cleanable patterns
  fn check_cleanable_pattern(&self, name: &str) -> String {
    // Direct pattern match
    if let Some(description) = CLEANABLE_PATTERNS.get(name) {
      return format!(
        "PATTERN MATCH: '{}' matches DevTidy pattern '{}' - {}. This CAN be safely deleted.",
        name, name, description
      );
    }

    // Wildcard pattern matching
    for (pattern, description) in CLEANABLE_PATTERNS.iter() {
      if pattern.contains('*') {
        let pattern_base = pattern.replace('*', "");
        if name.starts_with(&pattern_base) || name.contains(&pattern_base) {
          return format!("PATTERN MATCH: '{}' matches DevTidy wildcard pattern '{}' - {}. This CAN be safely deleted.", name, pattern, description);
        }
      }
    }

    // Extension matching for files
    if name.contains('.') {
      let extension = format!("*.{}", name.split('.').last().unwrap_or(""));
      if let Some(description) = CLEANABLE_PATTERNS.get(extension.as_str()) {
        return format!("PATTERN MATCH: '{}' matches DevTidy extension pattern '{}' - {}. This CAN be safely deleted.", name, extension, description);
      }
    }

    format!("NO PATTERN MATCH: '{}' does not match any DevTidy cleanable patterns. This should NOT be deleted as it's likely important project files.", name)
  }

  /// Get recent conversation context
  fn get_conversation_context(&self) -> String {
    if self.conversation_history.is_empty() {
      return "No previous conversation.".to_string();
    }

    let context = self
      .conversation_history
      .iter()
      .enumerate()
      .map(|(i, (user, ai))| format!("Previous exchange {}:\nUser: {}\nAI: {}", i + 1, user, ai))
      .collect::<Vec<_>>()
      .join("\n\n");

    format!("Recent conversation:\n{}", context)
  }
}

impl Default for DevTidyContext {
  fn default() -> Self {
    Self::new()
  }
}
