import SwiftUI

struct ConsoleLogView: View {
    let logs: [LogEntry] = [
        LogEntry(timestamp: Date(), level: .info, message: "App launched successfully"),
        LogEntry(timestamp: Date(), level: .debug, message: "View hierarchy initialized"),
        LogEntry(timestamp: Date(), level: .warning, message: "Memory usage above 70%"),
        LogEntry(timestamp: Date(), level: .error, message: "Failed to load resource: image_001.png"),
        LogEntry(timestamp: Date(), level: .info, message: "Network request completed")
    ]

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 4) {
                ForEach(logs, id: \.timestamp) { log in
                    HStack(alignment: .top, spacing: 8) {
                        Text(log.timestamp, style: .time)
                            .font(.system(.caption2, design: .monospaced))
                            .foregroundColor(.secondary)

                        Image(systemName: log.level.iconName)
                            .font(.caption2)
                            .foregroundColor(log.level.color)

                        Text(log.message)
                            .font(.system(.caption, design: .monospaced))
                            .foregroundColor(log.level == .error ? .red : .primary)

                        Spacer()
                    }
                    .padding(.vertical, 2)

                    if log.level == .error {
                        Rectangle()
                            .fill(Color.red.opacity(0.1))
                            .frame(height: 1)
                    }
                }
            }
            .padding()
        }
    }
}

struct LogEntry {
    let timestamp: Date
    let level: LogLevel
    let message: String
}

enum LogLevel {
    case debug, info, warning, error

    var iconName: String {
        switch self {
        case .debug: return "ladybug"
        case .info: return "info.circle"
        case .warning: return "exclamationmark.triangle"
        case .error: return "xmark.circle"
        }
    }

    var color: Color {
        switch self {
        case .debug: return .purple
        case .info: return .blue
        case .warning: return .orange
        case .error: return .red
        }
    }
}

#Preview {
    ConsoleLogView()
}
