import SwiftUI

struct NetworkLogView: View {
    let requests: [NetworkRequest] = [
        NetworkRequest(method: "GET", url: "https://api.example.com/users", status: "200 OK", duration: "120ms"),
        NetworkRequest(method: "POST", url: "https://api.example.com/events", status: "201 Created", duration: "85ms")
    ]

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 8) {
                ForEach(requests, id: \.url) { request in
                    HStack(alignment: .top) {
                        VStack(alignment: .leading, spacing: 4) {
                            HStack(spacing: 6) {
                                Text(request.method)
                                    .font(.system(.caption, design: .monospaced))
                                    .fontWeight(.bold)
                                    .foregroundColor(methodColor(request.method))

                                Text(request.url)
                                    .font(.system(.caption2, design: .monospaced))
                                    .foregroundColor(.primary)
                                    .lineLimit(1)
                            }

                            HStack(spacing: 12) {
                                Label(request.status, systemImage: "checkmark.circle")
                                    .font(.system(.caption2, design: .monospaced))
                                    .foregroundColor(statusColor(request.status))

                                Label(request.duration, systemImage: "clock")
                                    .font(.system(.caption2, design: .monospaced))
                                    .foregroundColor(.secondary)
                            }
                        }

                        Spacer()
                    }
                    .padding(.vertical, 8)
                    .padding(.horizontal, 10)
                    .background(Color(UIColor.secondarySystemBackground))
                    .cornerRadius(6)
                }
            }
            .padding()
        }
    }

    func methodColor(_ method: String) -> Color {
        switch method {
        case "GET": return .green
        case "POST": return .blue
        case "PUT": return .orange
        case "DELETE": return .red
        default: return .gray
        }
    }

    func statusColor(_ status: String) -> Color {
        if status.starts(with: "2") {
            return .green
        } else if status.starts(with: "4") {
            return .orange
        } else if status.starts(with: "5") {
            return .red
        }
        return .secondary
    }
}

struct NetworkRequest {
    let method: String
    let url: String
    let status: String
    let duration: String
}

#Preview {
    NetworkLogView()
}
